use elvpath_lib::catalog::load_embedded_catalog;
use elvpath_lib::models::{NetworkMode, NetworkProfile, OperatingSystem, PlanSelection};
use elvpath_lib::planner::build_install_plan;

#[test]
fn install_plan_orders_dependencies_before_selected_items() {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let selections = vec![PlanSelection {
        item_id: "docker".into(),
        version: Some("latest".into()),
    }];
    let profile = NetworkProfile::default();

    let plan = build_install_plan(&catalog, &selections, &profile, OperatingSystem::Windows)
        .expect("plan builds");

    let ordered_ids: Vec<_> = plan
        .items
        .iter()
        .map(|item| item.item_id.as_str())
        .collect();
    let git_index = ordered_ids
        .iter()
        .position(|id| *id == "git")
        .expect("git dependency");
    let docker_index = ordered_ids
        .iter()
        .position(|id| *id == "docker")
        .expect("docker selected");
    assert!(git_index < docker_index);
    assert!(plan.requires_admin);
    assert!(plan
        .steps
        .iter()
        .any(|step| step.permissions.requires_admin));
}

#[test]
fn mirror_profile_rewrites_download_urls_when_available() {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let selections = vec![PlanSelection {
        item_id: "nodejs".into(),
        version: Some("22".into()),
    }];
    let profile = NetworkProfile {
        mode: NetworkMode::ChinaMirror,
        proxy_url: None,
        mirror_region: Some("cn".into()),
    };

    let plan = build_install_plan(&catalog, &selections, &profile, OperatingSystem::Linux).unwrap();

    assert!(
        plan.steps.iter().any(|step| step
            .url
            .as_deref()
            .is_some_and(|url| url.contains("npmmirror.com"))),
        "expected at least one mirror URL in {:#?}",
        plan.steps
    );
}
