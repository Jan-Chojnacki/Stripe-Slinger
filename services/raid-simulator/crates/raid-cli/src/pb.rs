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

#[cfg(test)]
mod tests {
    use super::metrics;

    #[test]
    fn metrics_batch_has_defaults() {
        let batch = metrics::MetricsBatch::default();
        assert_eq!(batch.seq_no, 0);
        assert!(batch.disk_ops.is_empty());
        assert!(batch.raid_ops.is_empty());
    }
}
