/// Information about a Whisper model
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub name: &'static str,
    pub size_mb: u64,
    pub parameters: &'static str,
    pub description: &'static str,
    pub recommended: bool,
    pub hf_repo: &'static str,
    pub hf_revision: &'static str,
}

/// Registry of available Whisper models
pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "tiny",
        size_mb: 75,
        parameters: "39M",
        description: "Fastest, lowest accuracy",
        recommended: false,
        hf_repo: "openai/whisper-tiny",
        hf_revision: "main",
    },
    ModelInfo {
        name: "base",
        size_mb: 145,
        parameters: "74M",
        description: "Balanced for most use cases",
        recommended: true,
        hf_repo: "openai/whisper-base",
        hf_revision: "refs/pr/22",
    },
    ModelInfo {
        name: "small",
        size_mb: 466,
        parameters: "244M",
        description: "Better accuracy, slower",
        recommended: false,
        hf_repo: "openai/whisper-small",
        hf_revision: "main",
    },
    ModelInfo {
        name: "medium",
        size_mb: 1450,
        parameters: "769M",
        description: "High accuracy, much slower",
        recommended: false,
        hf_repo: "openai/whisper-medium",
        hf_revision: "main",
    },
    ModelInfo {
        name: "large",
        size_mb: 2900,
        parameters: "1550M",
        description: "Best accuracy, very slow",
        recommended: false,
        hf_repo: "openai/whisper-large-v3",
        hf_revision: "main",
    },
];

impl ModelInfo {
    /// Find model by name
    #[must_use]
    pub fn find(name: &str) -> Option<&'static Self> {
        MODELS.iter().find(|m| m.name == name)
    }

    /// Get all model names
    #[must_use]
    pub fn all_names() -> Vec<&'static str> {
        MODELS.iter().map(|m| m.name).collect()
    }

    /// Get recommended model
    #[must_use]
    pub fn recommended() -> &'static Self {
        MODELS.iter().find(|m| m.recommended).unwrap()
    }

    /// Find closest match using Levenshtein distance
    #[must_use]
    pub fn suggest(name: &str) -> Option<&'static str> {
        if name.is_empty() {
            return None;
        }

        MODELS
            .iter()
            .map(|m| (m.name, levenshtein_distance(name, m.name)))
            .min_by_key(|(_, dist)| *dist)
            .filter(|(_, dist)| *dist <= 2) // Only suggest if within 2 edits
            .map(|(model_name, _)| model_name)
    }
}

/// Calculate Levenshtein distance between two strings
#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for (i, c1) in s1_chars.iter().enumerate() {
        for (j, c2) in s2_chars.iter().enumerate() {
            let cost = usize::from(c1 != c2);
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_model() {
        assert!(ModelInfo::find("base").is_some());
        assert!(ModelInfo::find("tiny").is_some());
        assert!(ModelInfo::find("large").is_some());
        assert!(ModelInfo::find("invalid").is_none());
    }

    #[test]
    fn test_all_names() {
        let names = ModelInfo::all_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"tiny"));
        assert!(names.contains(&"base"));
        assert!(names.contains(&"small"));
        assert!(names.contains(&"medium"));
        assert!(names.contains(&"large"));
    }

    #[test]
    fn test_recommended() {
        let recommended = ModelInfo::recommended();
        assert_eq!(recommended.name, "base");
        assert!(recommended.recommended);
    }

    #[test]
    fn test_suggest() {
        assert_eq!(ModelInfo::suggest("basee"), Some("base"));
        assert_eq!(ModelInfo::suggest("bse"), Some("base"));
        assert_eq!(ModelInfo::suggest("tin"), Some("tiny"));
        assert_eq!(ModelInfo::suggest("smal"), Some("small"));
        assert_eq!(ModelInfo::suggest("larg"), Some("large"));
        assert_eq!(ModelInfo::suggest("invalid123"), None);
        assert_eq!(ModelInfo::suggest(""), None);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("base", "base"), 0);
        assert_eq!(levenshtein_distance("base", "basee"), 1);
        assert_eq!(levenshtein_distance("base", "bse"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_model_metadata() {
        let base = ModelInfo::find("base").unwrap();
        assert_eq!(base.name, "base");
        assert_eq!(base.size_mb, 145);
        assert_eq!(base.parameters, "74M");
        assert!(base.recommended);
    }
}
