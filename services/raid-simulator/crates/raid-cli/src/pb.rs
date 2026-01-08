#[allow(
    clippy::struct_excessive_bools,
    clippy::enum_variant_names,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::trivially_copy_pass_by_ref
)]
pub mod metrics {
    tonic::include_proto!("metrics.v1");
}
