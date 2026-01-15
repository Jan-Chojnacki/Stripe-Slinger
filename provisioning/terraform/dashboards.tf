resource "grafana_dashboard" "storage_dashboards" {
  for_each = fileset("${path.module}/../../observability/grafana/dashboards", "*.json")

  config_json = file("${path.module}/../../observability/grafana/dashboards/${each.value}")

  overwrite = true
}
