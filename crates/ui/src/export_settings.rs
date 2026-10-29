//! UI-only export configuration.
//!
//! The first milestone stores and validates settings but does not encode
//! anything. Codec and container are fixed to H.264 in MP4 so the form cannot
//! express combinations that a future encoder would not support.

/// Output resolution choices offered by the export dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportResolution {
    P720,
    P1080,
    P2160,
}

impl ExportResolution {
    pub const ALL: [Self; 3] = [Self::P720, Self::P1080, Self::P2160];

    pub fn label(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
            Self::P2160 => "2160p",
        }
    }

    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::P720 => (1280, 720),
            Self::P1080 => (1920, 1080),
            Self::P2160 => (3840, 2160),
        }
    }
}

/// Output frame-rate choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFrameRate {
    Fps24,
    Fps30,
    Fps60,
}

impl ExportFrameRate {
    pub const ALL: [Self; 3] = [Self::Fps24, Self::Fps30, Self::Fps60];

    pub fn frames_per_second(self) -> u32 {
        match self {
            Self::Fps24 => 24,
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fps24 => "24 fps",
            Self::Fps30 => "30 fps",
            Self::Fps60 => "60 fps",
        }
    }
}

/// User-selectable bitrate intent. No numeric encoder bitrate is fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitratePreset {
    Recommended,
    Higher,
    Custom,
}

impl BitratePreset {
    pub const ALL: [Self; 3] = [Self::Recommended, Self::Higher, Self::Custom];

    pub fn label(self) -> &'static str {
        match self {
            Self::Recommended => "Recommended",
            Self::Higher => "Higher",
            Self::Custom => "Custom",
        }
    }
}

/// Constraints for the custom bitrate field, in kilobits per second.
pub const CUSTOM_BITRATE_MIN_KBPS: u32 = 500;
pub const CUSTOM_BITRATE_MAX_KBPS: u32 = 200_000;
pub const MAX_FILENAME_CHARS: usize = 120;

/// A validation problem with the export form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportIssue {
    EmptyFilename,
    FilenameTooLong,
    InvalidFilenameCharacter,
    EmptyDirectory,
    CustomBitrateOutOfRange,
}

impl ExportIssue {
    pub fn message(self) -> &'static str {
        match self {
            Self::EmptyFilename => "Enter an output file name.",
            Self::FilenameTooLong => "The file name is too long.",
            Self::InvalidFilenameCharacter => "The file name contains an invalid character.",
            Self::EmptyDirectory => "Enter an output folder.",
            Self::CustomBitrateOutOfRange => "The custom bitrate is outside the supported range.",
        }
    }
}

/// Export form values, stored for the lifetime of a temporary editor session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSettings {
    pub filename: String,
    pub directory: String,
    pub resolution: ExportResolution,
    pub frame_rate: ExportFrameRate,
    pub bitrate: BitratePreset,
    pub custom_bitrate_kbps: u32,
    pub include_audio: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            filename: "cutcraft-export.mp4".to_owned(),
            directory: String::new(),
            resolution: ExportResolution::P1080,
            frame_rate: ExportFrameRate::Fps30,
            bitrate: BitratePreset::Recommended,
            custom_bitrate_kbps: 12_000,
            include_audio: true,
        }
    }
}

impl ExportSettings {
    /// Codec is fixed for this milestone.
    pub fn codec_label(&self) -> &'static str {
        "H.264"
    }

    /// Container is fixed for this milestone.
    pub fn container_label(&self) -> &'static str {
        "MP4"
    }

    /// Validate the form for a future encoder handoff.
    ///
    /// This does not check filesystem existence or write access; the UI shell
    /// performs no I/O.
    pub fn issues(&self) -> Vec<ExportIssue> {
        let mut issues = Vec::new();
        let filename = self.filename.trim();

        if filename.is_empty() {
            issues.push(ExportIssue::EmptyFilename);
        } else {
            if filename.chars().count() > MAX_FILENAME_CHARS {
                issues.push(ExportIssue::FilenameTooLong);
            }
            if filename
                .chars()
                .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
            {
                issues.push(ExportIssue::InvalidFilenameCharacter);
            }
        }

        if self.directory.trim().is_empty() {
            issues.push(ExportIssue::EmptyDirectory);
        }

        if self.bitrate == BitratePreset::Custom
            && !(CUSTOM_BITRATE_MIN_KBPS..=CUSTOM_BITRATE_MAX_KBPS)
                .contains(&self.custom_bitrate_kbps)
        {
            issues.push(ExportIssue::CustomBitrateOutOfRange);
        }

        issues
    }

    pub fn is_valid(&self) -> bool {
        self.issues().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_settings() -> ExportSettings {
        ExportSettings {
            directory: "C:\\Exports".to_owned(),
            ..ExportSettings::default()
        }
    }

    #[test]
    fn default_settings_need_an_output_folder() {
        let issues = ExportSettings::default().issues();

        assert_eq!(issues, vec![ExportIssue::EmptyDirectory]);
    }

    #[test]
    fn complete_settings_validate() {
        assert!(valid_settings().is_valid());
    }

    #[test]
    fn filename_validation_rejects_empty_long_and_reserved_characters() {
        let mut settings = valid_settings();
        settings.filename = "   ".to_owned();
        assert!(settings.issues().contains(&ExportIssue::EmptyFilename));

        settings.filename = "x".repeat(MAX_FILENAME_CHARS + 1);
        assert!(settings.issues().contains(&ExportIssue::FilenameTooLong));

        settings.filename = "bad/name.mp4".to_owned();
        assert!(settings
            .issues()
            .contains(&ExportIssue::InvalidFilenameCharacter));
    }

    #[test]
    fn empty_directory_is_invalid() {
        let mut settings = valid_settings();
        settings.directory = "  ".to_owned();

        assert_eq!(settings.issues(), vec![ExportIssue::EmptyDirectory]);
    }

    #[test]
    fn custom_bitrate_is_checked_only_for_custom_preset() {
        let mut settings = valid_settings();
        settings.bitrate = BitratePreset::Custom;
        settings.custom_bitrate_kbps = CUSTOM_BITRATE_MIN_KBPS - 1;
        assert!(settings
            .issues()
            .contains(&ExportIssue::CustomBitrateOutOfRange));

        settings.custom_bitrate_kbps = 8_000;
        assert!(settings.is_valid());

        settings.bitrate = BitratePreset::Recommended;
        settings.custom_bitrate_kbps = 0;
        assert!(settings.is_valid());
    }

    #[test]
    fn resolution_and_frame_rate_labels_are_stable() {
        assert_eq!(ExportResolution::P720.dimensions(), (1280, 720));
        assert_eq!(ExportResolution::P2160.label(), "2160p");
        assert_eq!(ExportFrameRate::Fps24.frames_per_second(), 24);
        assert_eq!(ExportFrameRate::Fps60.label(), "60 fps");
        assert_eq!(BitratePreset::Higher.label(), "Higher");
        assert_eq!(ExportSettings::default().codec_label(), "H.264");
        assert_eq!(ExportSettings::default().container_label(), "MP4");
    }
}
