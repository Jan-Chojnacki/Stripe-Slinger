resource "grafana_dashboard" "storage_dashboards" {
  for_each = fileset("${path.module}/../dashboards", "*.json")

  config_json = file("${path.module}/../dashboards/${each.value}")

  overwrite = true
}
