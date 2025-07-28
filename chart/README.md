# Custom DDNS Helm Chart

This Helm chart deploys the Custom DDNS service on a Kubernetes cluster using the Helm package manager.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.2.0+
- A container registry with the custom-ddns image

## Installing the Chart

To install the chart with the release name `my-custom-ddns`:

```bash
helm install my-custom-ddns ./chart
```

The command deploys custom-ddns on the Kubernetes cluster in the default configuration. The [Parameters](#parameters) section lists the parameters that can be configured during installation.

## Uninstalling the Chart

To uninstall/delete the `my-custom-ddns` deployment:

```bash
helm delete my-custom-ddns
```

## Parameters

### Global Parameters

| Name | Description | Value |
| ---- | ----------- | ----- |
| `replicaCount` | Number of custom-ddns replicas to deploy | `1` |
| `image.repository` | custom-ddns image repository | `ghcr.io/deimosfr/custom-ddns` |
| `image.tag` | custom-ddns image tag (immutable tags are recommended) | `""` |
| `image.pullPolicy` | custom-ddns image pull policy | `IfNotPresent` |
| `priorityClassName` | Priority class name for pod scheduling | `""` |
| `strategy.type` | Deployment strategy type | `Recreate` |

### Security Parameters

| Name | Description | Value |
| ---- | ----------- | ----- |
| `podSecurityContext.fsGroup` | Set custom-ddns pod's Security Context fsGroup | `1000` |
| `podSecurityContext.runAsUser` | Set custom-ddns pod's Security Context runAsUser | `1000` |
| `podSecurityContext.runAsNonRoot` | Set custom-ddns pod's Security Context runAsNonRoot | `true` |

### Resource Parameters

| Name | Description | Value |
| ---- | ----------- | ----- |
| `resources.limits.cpu` | The CPU limit for the custom-ddns containers | `100m` |
| `resources.limits.memory` | The memory limit for the custom-ddns containers | `128Mi` |
| `resources.requests.cpu` | The requested CPU for the custom-ddns containers | `10m` |
| `resources.requests.memory` | The requested memory for the custom-ddns containers | `64Mi` |

### Application Configuration

| Name | Description | Value |
| ---- | ----------- | ----- |
| `config.dns_records` | Array of DNS records to manage (includes API keys) | `[]` |

## Configuration and Installation

1. **Configure DNS Records with API Keys**: Edit the `values.yaml` file to configure your DNS records including API keys:

```yaml
config:
  dns_records:
    - name: "home-ipv4"
      source:
        check_interval_in_seconds: 300
        freebox:
          url: "http://mafreebox.freebox.fr"
          token: "your-freebox-token-here"
      domain:
        provider: "Cloudflare"
        domain_name: "example.com"
        record_name: "home"
        record_type: "A"
        record_ttl: 300
        api_key: "your-cloudflare-api-key-here"
```

2. **Install with custom values**:

```bash
helm install my-custom-ddns ./chart -f my-values.yaml
```

## Security

All configuration data including API keys and tokens are stored securely in a Kubernetes Secret. The configuration is base64-encoded and encrypted at rest by Kubernetes.

## Deployment Strategy

The chart uses a `Recreate` deployment strategy by default, which ensures:
- Clean shutdown of the existing pod before starting a new one
- No overlap between old and new instances
- Suitable for applications that shouldn't run multiple instances simultaneously
- Prevents potential conflicts with DNS record management

## Priority Class

You can optionally set a priority class for the pods to ensure proper scheduling:

```yaml
priorityClassName: "high-priority"
```

This is useful in clusters with resource constraints to ensure the DDNS service gets priority over less critical workloads.

## Monitoring

The chart includes liveness and readiness probes that check if the custom-ddns process is running. You can monitor the application using:

```bash
# Check pod status
kubectl get pods -l app.kubernetes.io/name=custom-ddns

# View logs
kubectl logs -l app.kubernetes.io/name=custom-ddns

# Check configuration (values will be base64 encoded)
kubectl get secret my-custom-ddns-config -o yaml
```

## Troubleshooting

### Common Issues

1. **Pod not starting**: Check if the image is accessible and the configuration is valid
2. **DNS updates failing**: Verify API keys and domain configurations
3. **Freebox connection issues**: Ensure the Freebox URL is accessible from the cluster

### Debug Commands

```bash
# Check pod events
kubectl describe pod <pod-name>

# View detailed logs
kubectl logs <pod-name> -f

# Verify configuration and secrets (values will be base64 encoded)
kubectl get secret <release-name>-config -o yaml
``` 