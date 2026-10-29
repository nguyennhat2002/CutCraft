//! Empty track rows shown by the timeline shell.

/// Domain of a timeline track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
}

/// Static description used by the shell; not persistent project state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackDescriptor {
    pub label: &'static str,
    pub name: &'static str,
    pub kind: TrackKind,
}

const TRACKS: [TrackDescriptor; 2] = [
    TrackDescriptor {
        label: "V1",
        name: "Video 1",
        kind: TrackKind::Video,
    },
    TrackDescriptor {
        label: "A1",
        name: "Audio 1",
        kind: TrackKind::Audio,
    },
];

pub fn tracks() -> &'static [TrackDescriptor] {
    &TRACKS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_has_one_video_and_one_audio_track() {
        let tracks = tracks();

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].kind, TrackKind::Video);
        assert_eq!(tracks[1].kind, TrackKind::Audio);
        assert_eq!(tracks[0].label, "V1");
        assert_eq!(tracks[1].label, "A1");
    }
}
