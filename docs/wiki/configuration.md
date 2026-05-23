# Configuration

vex can be customized through a TOML configuration file. The application looks for this file in the standard configuration directories for your platform:

- Linux: `$XDG_CONFIG_HOME/vex/config.toml` or `$HOME/.config/vex/config.toml`
- macOS: `$HOME/Library/Application Support/vex/config.toml`
- Windows: `{FOLDERID_RoamingAppData}\vex\config.toml`

## Configuration Options

### GitHub Settings

```toml
[github]
# Your GitHub personal access token (can also be set via GITHUB_TOKEN environment variable)
token = "your_token_here"

# Default repository to show issues/PRs from (format: "owner/repo")
default_repo = "owner/repo"

# Number of items to fetch per page (default: 30)
per_page = 30
```

### UI Settings

```toml
[ui]
# Theme to use: "default", "dark", "light"
theme = "default"

# Refresh interval in seconds (default: 30)
refresh_interval = 30

# Enable/disable mouse support
mouse_enabled = true

# Date format for displaying dates (uses chrono format strings)
date_format = "%Y-%m-%d %H:%M"
```

### Keybindings

```toml
[keybindings]
# Navigation
up = "k"
down = "j"
left = "h"
right = "l"
page_up = "PageUp"
page_down = "PageDown"
home = "Home"
end = "End"

# Actions
refresh = "r"
open_in_browser = "o"
create_issue = "n"
close_issue = "c"
reopen_issue = "R"
assign_to_me = "a"
add_label = "l"
```

### Database Settings

```toml
[database]
# Path to SQLite database file (default: platform-appropriate data directory)
# path = "/path/to/vex.db"

# Enable/disable automatic vacuuming
auto_vacuum = true

# Cache expiration time in hours (default: 24)
cache_expiration_hours = 24
```

## Example Configuration

Here's a complete example configuration file:

```toml
[github]
token = "ghp_your_actual_token_here"
default_repo = "owner/awesome-project"
per_page = 50

[ui]
theme = "dark"
refresh_interval = 15
mouse_enabled = true
date_format = "%Y-%m-%d %H:%M:%S"

[keybindings]
up = "k"
down = "j"
refresh = "r"
open_in_browser = "o"
create_issue = "n"

[database]
auto_vacuum = true
cache_expiration_hours = 12
```