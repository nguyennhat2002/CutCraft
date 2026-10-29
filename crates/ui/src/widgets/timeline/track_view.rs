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

const TRACKS: [TrackDescriptor; 6] = [
    TrackDescriptor {
        label: "V3",
        name: "Video 3",
        kind: TrackKind::Video,
    },
    TrackDescriptor {
        label: "V2",
        name: "Video 2",
        kind: TrackKind::Video,
    },
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
    TrackDescriptor {
        label: "A2",
        name: "Audio 2",
        kind: TrackKind::Audio,
    },
    TrackDescriptor {
        label: "A3",
        name: "Audio 3",
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
    fn shell_has_three_video_and_three_audio_tracks() {
        let tracks = tracks();

        assert_eq!(tracks.len(), 6);
        assert_eq!(tracks[0].kind, TrackKind::Video);
        assert_eq!(tracks[2].kind, TrackKind::Video);
        assert_eq!(tracks[3].kind, TrackKind::Audio);
        assert_eq!(tracks[5].kind, TrackKind::Audio);
        assert_eq!(tracks[0].label, "V3");
        assert_eq!(tracks[3].label, "A1");
    }
}
