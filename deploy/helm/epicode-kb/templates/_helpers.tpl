{{/* Expand the name of the chart. */}}
{{- define "epicode-kb.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/* Create chart name and version */}}
{{- define "epicode-kb.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/* Common labels */}}
{{- define "epicode-kb.labels" -}}
helm.sh/chart: {{ include "epicode-kb.chart" . }}
{{ include "epicode-kb.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/* Selector labels */}}
{{- define "epicode-kb.selectorLabels" -}}
app.kubernetes.io/name: {{ include "epicode-kb.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
