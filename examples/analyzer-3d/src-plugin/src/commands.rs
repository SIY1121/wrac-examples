//! Registration of commands callable from the WebView frontend.
//!
//! From Rust's perspective this module is the contract with the TypeScript UI.
//! When renaming commands or changing payload shapes, update the `invoke(...)` calls
//! and subscriptions in `src-gui` at the same time.

use std::rc::Rc;
use std::sync::Arc;

use serde_json::json;
use wrac_clap_adapter::{HostGuiResizeRequester, HostParameterEditNotifier};
use wrac_wxp_gui::WxpGuiResizeHandle;
use wxp::{Channel, WxpCommandHandler};

use crate::gui::{GuiStateNotifier, GuiSubscriptionId, editor_page_payload};
use crate::state::{EditorPage, ProjectStateStore, SharedState};

mod resize;

use resize::register_resize_commands;

/// Registers commands callable from the WebView frontend with the [`WxpCommandHandler`].
///
/// The frontend (TypeScript in `src-gui`) invokes these commands using calls like
/// `invoke("set_parameter_value", { parameterId, value })`.
pub(crate) fn register_commands(
    command_handler: Rc<WxpCommandHandler>,
    project_state: Arc<ProjectStateStore>,
    shared: Arc<SharedState>,
    gui_notifier: Arc<GuiStateNotifier>,
    host_parameter_edit_notifier: Arc<dyn HostParameterEditNotifier>,
    host_gui_resize_requester: Arc<dyn HostGuiResizeRequester>,
    gui_resize_handle: WxpGuiResizeHandle,
) {
    // The WebView console is often invisible inside a DAW. Bridge frontend logs to the
    // plugin's logger so GUI initialisation progress is visible in native log output.
    command_handler.register_sync("write_to_log", move |ctx| {
        let message = ctx.arg::<String>("message").map_err(|e| e.to_string())?;
        log::info!("frontend: {message}");
        Ok::<_, String>(json!({ "ok": true }))
    });

    // Editor page is project state unrelated to audio. It lives in a separate store from
    // the SharedState read by the audio thread and is merged with the parameter snapshot
    // at save time.
    {
        let project_state = project_state.clone();
        command_handler.register_sync("get_editor_page", move |_| {
            Ok::<_, String>(editor_page_payload(project_state.editor_page()))
        });
    }

    {
        let project_state = project_state.clone();
        let gui_notifier = gui_notifier.clone();
        command_handler.register_sync("set_editor_page", move |ctx| {
            let page = ctx.arg::<String>("page").map_err(|e| e.to_string())?;
            let editor_page =
                EditorPage::from_str(&page).ok_or_else(|| "invalid editor page".to_string())?;
            project_state.set_editor_page(editor_page);
            gui_notifier.notify_editor_page(editor_page);
            Ok::<_, String>(editor_page_payload(editor_page))
        });
    }

    // Starts a subscription that receives analyzer result.
    // `channel` is a callback channel created on the JS side; the plugin pushes value
    // changes into it. The returned `subscriptionId` identifies the subscription so the
    // JS side can unsubscribe precisely at cleanup, without cancelling subscriptions it
    // didn't create.
    {
        let gui_notifier = gui_notifier.clone();
        let shared = shared.clone();
        command_handler.register_sync("subscribe_spectrogram", move |ctx| {
            let channel = ctx.arg::<Channel>("channel").map_err(|e| e.to_string())?;
            shared.add_spectrogram_subscription();
            let subscription_id = gui_notifier.subscribe_spectrogram(channel);
            Ok::<_, String>(json!({
                "ok": true,
                "subscriptionId": subscription_id.get(),
            }))
        });
    }

    {
        let gui_notifier = gui_notifier.clone();
        let shared = shared.clone();
        command_handler.register_sync("subscribe_imager", move |ctx| {
            let channel = ctx.arg::<Channel>("channel").map_err(|e| e.to_string())?;
            shared.add_imager_subscription();
            let subscription_id = gui_notifier.subscribe_imager(channel);
            Ok::<_, String>(json!({
                "ok": true,
                "subscriptionId": subscription_id.get(),
            }))
        });
    }

    // Cancels a subscription. If the given ID is not registered this is a no-op.
    // Using an explicit ID prevents a delayed, stale cleanup from accidentally cancelling
    // a subscription that was created later.
    {
        let gui_notifier = gui_notifier.clone();
        let shared = shared.clone();
        command_handler.register_sync("unsubscribe_spectrogram", move |ctx| {
            let subscription_id = ctx
                .arg::<u64>("subscriptionId")
                .map_err(|e| e.to_string())?;
            shared.remove_spectrogram_subscription();
            gui_notifier.unsubscribe(GuiSubscriptionId::from_raw(subscription_id));
            Ok::<_, String>(json!({ "ok": true }))
        });
    }

    {
        let gui_notifier = gui_notifier.clone();
        let shared = shared.clone();
        command_handler.register_sync("unsubscribe_imager", move |ctx| {
            let subscription_id = ctx
                .arg::<u64>("subscriptionId")
                .map_err(|e| e.to_string())?;
            shared.remove_imager_subscription();
            gui_notifier.unsubscribe(GuiSubscriptionId::from_raw(subscription_id));
            Ok::<_, String>(json!({ "ok": true }))
        });
    }

    // Sets the FFT size used by the spectrogram.
    {
        let shared = shared.clone();
        command_handler.register_sync("set_fft_size", move |ctx| {
            let fft_size = ctx.arg::<u32>("fftSize").map_err(|e| e.to_string())?;
            shared.set_fft_size(fft_size);
            Ok::<_, String>(json!({ "ok": true }))
        });
    }

    command_handler.register_sync("focus_host_window", move |ctx| {
        ctx.webview()
            .post_focus_parent()
            .map_err(|e| format!("focus_parent failed: {e}"))?;
        Ok::<_, String>(json!({ "ok": true }))
    });

    register_resize_commands(
        &command_handler,
        host_gui_resize_requester,
        gui_resize_handle,
    );
}
