/// Utility functions for the berg-operator
/// Generate a namespace name from an owner ID
pub fn generate_namespace_name(namespace_prefix: &str, challenge_name: &str, id: &str) -> String {
    let max_chall_name_len = 63 - (namespace_prefix.len() + id.len() + 2);
    let challenge_name = if challenge_name.len() > max_chall_name_len {
        &challenge_name[..max_chall_name_len]
    } else {
        challenge_name
    };
    format!("{}-{}-{}", namespace_prefix, challenge_name, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_name_generation() {
        let owner_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let expected = "ci-nginx-a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        assert_eq!(generate_namespace_name("ci", "nginx", owner_id), expected);
    }
}
