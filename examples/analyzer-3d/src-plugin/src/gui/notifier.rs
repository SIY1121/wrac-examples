use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use novonotes_run_loop::{RunLoop, RunLoopSender};
use parking_lot::Mutex;
use serde_json::json;
use wxp::Channel;

use crate::state::EditorPage;

/// Outbound notification channel that pushes GUI state to the WebView. The caller decides when to notify.
pub(crate) struct GuiStateNotifier {
    next_subscription_id: AtomicU64,
    subscriptions: Mutex<HashMap<GuiSubscriptionId, GuiSubscription>>,
}

/// Registration record for a single WebView subscriber.
///
/// Separating `kind` (which stream) from `channel` (the destination) lets parameters,
/// meters, and analysers be subscribed and unsubscribed independently, and prevents a
/// stale cleanup from accidentally cancelling an unrelated subscription.
#[derive(Clone)]
struct GuiSubscription {
    kind: GuiSubscriptionKind,
    // Run loop sender for dispatching notifications back to the UI thread.
    sender: RunLoopSender,
    // Channel for sending values to the JS subscriber in the WebView.
    channel: Channel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GuiSubscriptionId(u64);

impl GuiSubscriptionId {
    pub(crate) fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

/// Subscription kind. Add a variant when adding meter or analyser streams, and deliver
/// to only matching subscriptions in `notify_*` — this design routes by kind rather than
/// multiplying channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiSubscriptionKind {
    EditorPage,
    Spectrogram,
    Imager,
}

impl GuiStateNotifier {
    pub(super) fn new() -> Self {
        Self {
            next_subscription_id: AtomicU64::new(1),
            subscriptions: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn subscribe_editor_page(&self, channel: Channel) -> GuiSubscriptionId {
        self.subscribe(GuiSubscriptionKind::EditorPage, channel)
    }

    pub(crate) fn subscribe_spectrogram(&self, channel: Channel) -> GuiSubscriptionId {
        self.subscribe(GuiSubscriptionKind::Spectrogram, channel)
    }

    pub(crate) fn subscribe_imager(&self, channel: Channel) -> GuiSubscriptionId {
        self.subscribe(GuiSubscriptionKind::Imager, channel)
    }

    fn subscribe(&self, kind: GuiSubscriptionKind, channel: Channel) -> GuiSubscriptionId {
        // IDs are assigned independently of wxp's Channel IDs so that transport and
        // subscription lifecycle can be managed separately.
        let id = GuiSubscriptionId(self.next_subscription_id.fetch_add(1, Ordering::Relaxed));
        self.subscriptions.lock().insert(
            id,
            GuiSubscription {
                kind,
                sender: RunLoop::sender(),
                channel,
            },
        );
        id
    }

    pub(crate) fn unsubscribe(&self, id: GuiSubscriptionId) {
        self.subscriptions.lock().remove(&id);
    }

    pub(crate) fn clear_subscriptions(&self) {
        self.subscriptions.lock().clear();
    }

    pub(crate) fn notify_editor_page(&self, editor_page: EditorPage) {
        self.notify(
            GuiSubscriptionKind::EditorPage,
            editor_page_payload(editor_page),
        );
    }

    pub(crate) fn notify_spectrogram(&self, payload: &Vec<f32>) {
        self.notify(GuiSubscriptionKind::Spectrogram, serde_json::json!(payload));
    }

    pub(crate) fn notify_imager(&self, payload: &Vec<f32>) {
        self.notify(GuiSubscriptionKind::Imager, serde_json::json!(payload));
    }

    fn notify(&self, kind: GuiSubscriptionKind, payload: serde_json::Value) {
        // Clone the delivery targets before releasing the lock so that a re-entrant
        // call from a recipient cannot deadlock.
        let subscriptions: Vec<_> = self
            .subscriptions
            .lock()
            .values()
            .filter(|subscription| subscription.kind == kind)
            .cloned()
            .collect();
        if subscriptions.is_empty() {
            // No subscribers when the GUI is closed; nothing to do.
            return;
        }

        for subscription in subscriptions {
            let payload = payload.clone();
            // WebView channels may only be touched on the same UI thread as the GUI
            // runtime. Sending directly from a host or audio thread would violate thread
            // affinity, so always dispatch back through the run loop first.
            subscription.sender.send(move || {
                let _ = subscription.channel.send(payload);
            });
        }
    }
}

pub(crate) fn editor_page_payload(editor_page: EditorPage) -> serde_json::Value {
    json!({
        "type": "editor-page",
        "page": editor_page.as_str(),
    })
}
