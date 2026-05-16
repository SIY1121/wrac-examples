use wrac_clap_adapter::{NoteDialects, NotePortInfo, PluginNotePorts};

// Note ports are separate from audio ports in CLAP. Declaring this input port is what
// lets a host route MIDI/note data into the process event stream.
pub(super) struct WracSineSynthNotePorts;

impl WracSineSynthNotePorts {
    pub(super) fn new() -> Self {
        Self
    }
}

impl PluginNotePorts for WracSineSynthNotePorts {
    fn note_port_count(&self, is_input: bool) -> u32 {
        if is_input { 1 } else { 0 }
    }

    fn note_port_info(&self, index: u32, is_input: bool) -> Option<NotePortInfo> {
        (index == 0 && is_input).then_some(NotePortInfo {
            id: 1,
            // Prefer CLAP notes because they carry structured note data, but advertise
            // MIDI too so hosts/wrappers with MIDI-oriented routing can still connect.
            supported_dialects: NoteDialects::CLAP.union(NoteDialects::MIDI),
            preferred_dialect: NoteDialects::CLAP,
            name: "MIDI In",
        })
    }
}
