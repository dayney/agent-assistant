use serde_json::Value;

pub(crate) fn merge_owned_keys(
    existing: &mut Value,
    desired: &Value,
    owned_keys: &[&str],
) -> Result<(), String> {
    let existing = existing
        .as_object_mut()
        .ok_or_else(|| "existing JSON value must be an object".to_string())?;
    let desired = desired
        .as_object()
        .ok_or_else(|| "desired JSON value must be an object".to_string())?;
    for key in owned_keys {
        if let Some(value) = desired.get(*key) {
            existing.insert((*key).to_string(), value.clone());
        } else {
            existing.remove(*key);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::merge_owned_keys;
    use serde_json::json;

    #[test]
    fn owned_keys_change_without_losing_foreign_data() {
        let mut existing = json!({
            "owned": {"old": true},
            "foreign": {"custom": 9007199254740993_u64}
        });
        let desired = json!({"owned": {"new": true}});

        merge_owned_keys(&mut existing, &desired, &["owned"]).unwrap();

        assert_eq!(existing["owned"], json!({"new": true}));
        assert_eq!(existing["foreign"]["custom"], json!(9007199254740993_u64));
    }

    #[test]
    fn absent_owned_key_is_removed() {
        let mut existing = json!({"owned": 1, "foreign": 2});
        merge_owned_keys(&mut existing, &json!({}), &["owned"]).unwrap();
        assert_eq!(existing, json!({"foreign": 2}));
    }
}
