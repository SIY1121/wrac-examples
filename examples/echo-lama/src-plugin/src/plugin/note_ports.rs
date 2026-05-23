use wrac_clap_adapter::{NoteDialects, NotePortInfo, PluginNotePorts};

/// One MIDI/CLAP note input port. Required for the host to route MIDI tracks to a synth.
pub(super) struct EchoLamaNotePorts;

impl EchoLamaNotePorts {
    pub(super) fn new() -> Self {
        Self
    }
}

impl PluginNotePorts for EchoLamaNotePorts {
    fn note_port_count(&self, is_input: bool) -> u32 {
        if is_input { 1 } else { 0 }
    }

    fn note_port_info(&self, index: u32, is_input: bool) -> Option<NotePortInfo> {
        if !is_input || index != 0 {
            return None;
        }
        Some(NotePortInfo {
            id: 1,
            // Accept both the native CLAP note dialect and traditional MIDI bytes — hosts
            // pick whichever fits their event pipeline.
            supported_dialects: NoteDialects::CLAP.union(NoteDialects::MIDI),
            preferred_dialect: NoteDialects::CLAP,
            name: "Note In",
        })
    }
}
