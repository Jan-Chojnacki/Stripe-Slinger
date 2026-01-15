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

  # =======================================================
  # RULE 1: RAID Degraded State
  # =======================================================
  rule {
    name      = "RAID Degraded State"
    condition = "C"
    for       = "1m"

    no_data_state  = "OK"
    exec_err_state = "Error"

    # KROK A: Query
    data {
      ref_id         = "A"
      datasource_uid = data.grafana_data_source.prometheus.uid
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        expr          = "raid_degraded_state"
        intervalMs    = 1000
        maxDataPoints = 43200
        refId         = "A"
      })
    }

    data {
      ref_id         = "B"
      datasource_uid = "__expr__"
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        conditions = [{
          evaluator = { params = [], type = "gt" }
          operator  = { type = "and" }
          query     = { params = [] }
          reducer   = { params = [], type = "last" }
          type      = "query"
        }]
        datasource = { type = "__expr__", uid = "__expr__" }
        expression = "A"
        reducer    = "last"
        refId      = "B"
        type       = "reduce"
      })
    }

    data {
      ref_id         = "C"
      datasource_uid = "__expr__"
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        datasource = { type = "__expr__", uid = "__expr__" }
        expression = "$B > 0"
        refId      = "C"
        type       = "math"
      })
    }

    annotations = {
      summary = "CRITICAL: RAID array {{ if $labels.raid }}{{ $labels.raid }}{{ else }}unknown{{ end }} is degraded"
    }
    labels = { severity = "critical" }
  }

  rule {
    name      = "High Disk Write Latency"
    condition = "C"
    for       = "3m"

    no_data_state = "OK"

    data {
      ref_id         = "A"
      datasource_uid = data.grafana_data_source.prometheus.uid
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        expr          = "histogram_quantile(0.95, sum by (le, disk_id, raid) (rate(raid_write_latency_seconds_bucket[5m])))"
        intervalMs    = 1000
        maxDataPoints = 43200
        refId         = "A"
      })
    }

    data {
      ref_id         = "B"
      datasource_uid = "__expr__"
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        conditions = [{
          evaluator = { params = [], type = "gt" }
          operator  = { type = "and" }
          query     = { params = [] }
          reducer   = { params = [], type = "last" }
          type      = "query"
        }]
        datasource = { type = "__expr__", uid = "__expr__" }
        expression = "A"
        reducer    = "last"
        refId      = "B"
        type       = "reduce"
      })
    }

    data {
      ref_id         = "C"
      datasource_uid = "__expr__"
      relative_time_range {
        from = 300
        to   = 0
      }
      model = jsonencode({
        datasource = { type = "__expr__", uid = "__expr__" }
        expression = "$B > 0.5"
        refId      = "C"
        type       = "math"
      })
    }

    annotations = {
      summary = "Warning: High latency on disk {{ $labels.disk_id }} in array {{ $labels.raid }}"
    }
    labels = { severity = "warning" }
  }
}
