use elvpath_lib::catalog::{load_embedded_catalog, validate_catalog};

#[test]
fn embedded_catalog_contains_required_categories_and_seed_tools() {
    let catalog = load_embedded_catalog().expect("catalog loads");

    let categories: Vec<_> = catalog
        .categories
        .iter()
        .map(|category| category.id.as_str())
        .collect();
    assert!(categories.contains(&"languages"));
    assert!(categories.contains(&"containers"));
    assert!(categories.contains(&"editors"));
    assert!(categories.contains(&"devops"));
    assert!(categories.contains(&"databases"));

    let ids: Vec<_> = catalog.items.iter().map(|item| item.id.as_str()).collect();
    for expected in [
        "rust", "nodejs", "python", "git", "docker", "vscode", "java", "go",
    ] {
        assert!(ids.contains(&expected), "missing seed tool {expected}");
    }
}

#[test]
fn catalog_validation_rejects_items_without_platform_steps() {
    let mut catalog = load_embedded_catalog().expect("catalog loads");
    catalog.items[0].install_steps.clear();

    let errors = validate_catalog(&catalog).expect_err("catalog should be invalid");
    assert!(
        errors.iter().any(|error| error.contains("install step")),
        "expected install step validation error, got {errors:?}"
    );
}
