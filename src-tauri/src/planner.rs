use std::collections::{HashMap, HashSet};

use crate::models::{
    Catalog, CatalogItem, InstallPlan, NetworkMode, NetworkProfile, OperatingSystem, PlanSelection,
    PlannedItem, PlannedStep,
};

pub fn build_install_plan(
    catalog: &Catalog,
    selections: &[PlanSelection],
    network: &NetworkProfile,
    platform: OperatingSystem,
) -> anyhow::Result<InstallPlan> {
    let items_by_id: HashMap<_, _> = catalog
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let selected_ids: HashSet<_> = selections
        .iter()
        .map(|selection| selection.item_id.as_str())
        .collect();
    let version_by_id: HashMap<_, _> = selections
        .iter()
        .map(|selection| (selection.item_id.as_str(), selection.version.clone()))
        .collect();

    let mut ordered = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for selection in selections {
        visit_item(
            &selection.item_id,
            &items_by_id,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }

    let mut planned_items = Vec::new();
    let mut steps = Vec::new();
    let mut warnings = Vec::new();

    for item in ordered {
        let version = version_by_id
            .get(item.id.as_str())
            .and_then(|version| version.clone())
            .or_else(|| {
                item.versions
                    .iter()
                    .find(|version| version.is_default)
                    .map(|version| version.id.clone())
            })
            .unwrap_or_else(|| "latest".to_string());

        planned_items.push(PlannedItem {
            item_id: item.id.clone(),
            name: item.name.clone(),
            version,
            dependency: !selected_ids.contains(item.id.as_str()),
        });

        let platform_steps: Vec<_> = item
            .install_steps
            .iter()
            .filter(|step| step.platforms.contains(&platform))
            .collect();

        if platform_steps.is_empty() {
            warnings.push(format!(
                "{} has no install step for {:?}",
                item.id, platform
            ));
        }

        for step in platform_steps {
            let url = match network.mode {
                NetworkMode::ChinaMirror => step.mirror_url.clone().or_else(|| step.url.clone()),
                NetworkMode::Official | NetworkMode::CustomProxy => step.url.clone(),
            };
            steps.push(PlannedStep {
                id: format!("{}:{}", item.id, step.id),
                item_id: item.id.clone(),
                title: step.title.clone(),
                kind: step.kind.clone(),
                source: step.source.clone(),
                url,
                checksum: step.checksum.clone(),
                command: step.command.clone(),
                args: step.args.clone(),
                permissions: step.permissions.clone(),
                config_changes: item.config_steps.clone(),
            });
        }
    }

    let requires_admin = steps.iter().any(|step| step.permissions.requires_admin);

    Ok(InstallPlan {
        id: stable_plan_id(selections, &platform),
        platform,
        network: network.clone(),
        items: planned_items,
        steps,
        requires_admin,
        warnings,
    })
}

fn visit_item<'a>(
    item_id: &str,
    items_by_id: &HashMap<&str, &'a CatalogItem>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<&'a CatalogItem>,
) -> anyhow::Result<()> {
    if visited.contains(item_id) {
        return Ok(());
    }
    if !visiting.insert(item_id.to_string()) {
        anyhow::bail!("dependency cycle detected at {item_id}");
    }

    let item = items_by_id
        .get(item_id)
        .ok_or_else(|| anyhow::anyhow!("unknown catalog item '{item_id}'"))?;

    for dependency in &item.dependencies {
        visit_item(dependency, items_by_id, visiting, visited, ordered)?;
    }

    visiting.remove(item_id);
    visited.insert(item_id.to_string());
    ordered.push(item);
    Ok(())
}

fn stable_plan_id(selections: &[PlanSelection], platform: &OperatingSystem) -> String {
    let ids = selections
        .iter()
        .map(|selection| selection.item_id.as_str())
        .collect::<Vec<_>>()
        .join("-");
    format!("{platform:?}-{ids}")
}
