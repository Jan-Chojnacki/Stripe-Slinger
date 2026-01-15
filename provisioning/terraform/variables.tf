variable "grafana_url" {
  description = "The root URL of your Grafana Cloud instance"
  type        = string
}

variable "grafana_token" {
  description = "Service Account Token with Admin/Editor permissions"
  type        = string
  sensitive   = true
}

variable "alert_email_address" {
  description = "Email address to receive critical alerts"
  type        = string
  sensitive   = true
}

variable "prometheus_name" {
  description = "The name of the Prometheus data source in Grafana"
  type        = string
}
