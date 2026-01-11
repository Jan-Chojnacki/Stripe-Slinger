data "grafana_data_source" "prometheus" {
  name = var.prometheus_name
}

resource "grafana_folder" "storage_alerts" {
  title = "Storage System Alerts"
}

resource "grafana_contact_point" "email_admin" {
  name = "Email Admin Team"
  email {
    addresses = [var.alert_email_address]
  }
}

resource "grafana_notification_policy" "main_policy" {
  group_by      = ["alertname"]
  contact_point = grafana_contact_point.email_admin.name
}

resource "grafana_rule_group" "raid_alerts" {
  name             = "RAID Status Rules"
  folder_uid       = grafana_folder.storage_alerts.uid
  interval_seconds = 60

  rule {
    name      = "RAID Degraded State"
    condition = "A"
    for       = "1m"

    no_data_state  = "OK"
    exec_err_state = "Error"

    data {
      ref_id         = "A"
      datasource_uid = data.grafana_data_source.prometheus.uid

      relative_time_range {
        from = 300
        to   = 0
      }

      model = jsonencode({
        datasourceUid = data.grafana_data_source.prometheus.uid
        expr          = "raid_degraded_state > 0"
        refId         = "A"
        type          = "prometheus"
      })
    }

    annotations = {
      summary = "CRITICAL: RAID array {{ if $labels.raid }}{{ $labels.raid }}{{ else }}unknown{{ end }} is degraded"
    }
    labels = { severity = "critical" }
  }

  rule {
    name      = "High Disk Write Latency"
    condition = "A"
    for       = "3m"

    no_data_state  = "OK"

    data {
      ref_id         = "A"
      datasource_uid = data.grafana_data_source.prometheus.uid

      relative_time_range {
        from = 300
        to   = 0
      }

      model = jsonencode({
        datasourceUid = data.grafana_data_source.prometheus.uid
        expr          = "histogram_quantile(0.95, sum by (le, disk_id, raid) (rate(raid_write_latency_seconds_bucket[5m]))) > 0.5"
        refId         = "A"
        type          = "prometheus"
      })
    }

    annotations = {
      summary = "Warning: High latency on disk {{ $labels.disk_id }} in array {{ $labels.raid }}"
    }
    labels = { severity = "warning" }
  }
}
