use rand::Rng;

/// Substitute {entropy} placeholder in a path with 12 random hex characters
pub fn substitute_entropy(path: &str) -> String {
    if !path.contains("{entropy}") {
        return path.to_string();
    }

    let entropy: String = (0..12)
        .map(|_| format!("{:x}", rand::thread_rng().gen_range(0..16)))
        .collect();

    path.replace("{entropy}", &entropy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_substitution() {
        let path = "/home/ctf/{entropy}/flag.txt";
        let result = substitute_entropy(path);
        assert!(result.contains("/home/ctf/"));
        assert!(!result.contains("{entropy}"));
        // Extract the entropy part
        let parts: Vec<&str> = result.split('/').collect();
        assert_eq!(parts[3].len(), 12);
        // Should be all hex digits
        assert!(parts[3].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_no_entropy() {
        let path = "/home/ctf/flag.txt";
        assert_eq!(substitute_entropy(path), path);
    }
}
