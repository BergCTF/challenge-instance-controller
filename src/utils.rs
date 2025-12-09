/// Utility functions for the berg-operator

/// Generate a namespace name from an owner ID
pub fn generate_namespace_name(owner_id: &str) -> String {
    format!("challenge-{}", owner_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_name_generation() {
        let owner_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let expected = "challenge-a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        assert_eq!(generate_namespace_name(owner_id), expected);
    }
}
