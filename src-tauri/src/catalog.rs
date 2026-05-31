use std::collections::HashSet;

use anyhow::Context;

use crate::models::{Catalog, SourceKind};

const EMBEDDED_CATALOG: &str = include_str!("../catalog/catalog.json");

pub fn load_embedded_catalog() -> anyhow::Result<Catalog> {
    let catalog: Catalog =
        serde_json::from_str(EMBEDDED_CATALOG).context("failed to parse embedded catalog")?;
    validate_catalog(&catalog).map_err(|errors| anyhow::anyhow!(errors.join("; ")))?;
    Ok(catalog)
}

pub fn validate_catalog(catalog: &Catalog) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let category_ids: HashSet<_> = catalog
        .categories
        .iter()
        .map(|category| category.id.as_str())
        .collect();
    let mut item_ids = HashSet::new();

    if catalog.categories.is_empty() {
        errors.push("catalog must define at least one category".to_string());
    }

    for item in &catalog.items {
        if item.id.trim().is_empty() {
            errors.push("catalog item has empty id".to_string());
        }
        if !item_ids.insert(item.id.as_str()) {
            errors.push(format!("duplicate catalog item id '{}'", item.id));
        }
        if !category_ids.contains(item.category.as_str()) {
            errors.push(format!(
                "item '{}' references missing category '{}'",
                item.id, item.category
            ));
        }
        if item.supported_platforms.is_empty() {
            errors.push(format!(
                "item '{}' must support at least one platform",
                item.id
            ));
        }
        if item.install_steps.is_empty() {
            errors.push(format!(
                "item '{}' must define at least one install step",
                item.id
            ));
        }
        if !item
            .sources
            .iter()
            .any(|source| source.kind == SourceKind::Official)
        {
            errors.push(format!(
                "item '{}' must include an official source",
                item.id
            ));
        }
        for dependency in &item.dependencies {
            if !catalog
                .items
                .iter()
                .any(|candidate| &candidate.id == dependency)
            {
                errors.push(format!(
                    "item '{}' depends on unknown item '{}'",
                    item.id, dependency
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
