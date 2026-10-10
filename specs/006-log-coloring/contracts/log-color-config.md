# Environment Configuration Contract: Log Coloring

| Name | Accepted behavior | Default |
|---|---|---|
| `EXCHANGE_API_LOG_COLOR` | Values `false`, `0`, `no`, and `off`, in any letter case, disable ANSI coloring. Any other value enables coloring. | Enabled when absent |

The setting is read at application startup and applies process-wide. Disabling colors removes ANSI color and formatting control sequences while preserving log messages and severity information. It does not change log filtering or message selection.
