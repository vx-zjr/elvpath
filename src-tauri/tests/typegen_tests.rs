use elvpath_lib::catalog::load_embedded_catalog;
use elvpath_lib::typegen::{
    generate_typescript_bindings, validate_generated_shape_against_catalog,
};

#[test]
fn generated_types_include_catalog_and_plan_contracts() {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let typescript = generate_typescript_bindings();

    assert!(validate_generated_shape_against_catalog(&catalog));
    assert!(typescript.contains("export interface CatalogItem"));
    assert!(typescript.contains("export interface PlannedStep"));
    assert!(typescript.contains("export interface DetectionResult"));
    assert!(typescript.contains("export type NetworkMode"));
}
