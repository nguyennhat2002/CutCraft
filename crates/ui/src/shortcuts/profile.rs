//! Active shortcut profile identity.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShortcutProfile {
    #[default]
    Default,
    Custom,
}

impl ShortcutProfile {
    pub const ALL: [Self; 2] = [Self::Default, Self::Custom];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "CutCraft Default",
            Self::Custom => "Custom",
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Custom => "custom",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "default" | "cutcraft default" => Some(Self::Default),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_names_round_trip() {
        for profile in ShortcutProfile::ALL {
            assert_eq!(ShortcutProfile::parse(profile.as_str()), Some(profile));
        }
        assert_eq!(
            ShortcutProfile::parse("custom "),
            Some(ShortcutProfile::Custom)
        );
        assert_eq!(ShortcutProfile::parse("missing"), None);
        assert_eq!(ShortcutProfile::Default.label(), "CutCraft Default");
    }
}
