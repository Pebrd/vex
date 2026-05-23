# Usage

This guide covers how to use vex for interacting with GitHub issues and pull requests.

## Getting Started

To run vex, you need to provide your GitHub personal access token. You can do this in two ways:

1. As a command-line argument:
   ```bash
   vex --token YOUR_GITHUB_TOKEN
   ```

2. As an environment variable:
   ```bash
   export GITHUB_TOKEN=YOUR_GITHUB_TOKEN
   vex
   ```

## Main Interface

When you start vex, you'll see the main interface divided into several sections:

- **Top Bar**: Shows the current repository, refresh status, and help
- **Main Panel**: Displays the list of issues or pull requests
- **Bottom Bar**: Shows available actions and keybindings
- **Right Panel** (when expanded): Shows detailed information about the selected item

## Navigation

Use these keys to navigate through the interface:

- `k` / `Up Arrow`: Move up
- `j` / `Down Arrow`: Move down
- `h` / `Left Arrow`: Move left (in split views)
- `l` / `Right Arrow`: Move right (in split views)
- `PageUp` / `PageDown`: Page up/down
- `Home` / `End`: Go to beginning/end of list
- `g`: Go to first item
- `G`: Go to last item

## Filtering and Sorting

vex supports filtering and sorting of issues and pull requests:

- `/`: Activate filter input
- `Esc`: Clear filter
- `s`: Open sort menu
- `f`: Open filter menu

In the filter menu, you can filter by:
- Author
- Labels
- Milestone
- State (open/closed)
- Assignee

In the sort menu, you can sort by:
- Created date
- Updated date
- Comments
- Reactions

## Actions

Perform actions on selected issues or pull requests:

- `o`: Open in browser
- `r`: Refresh data
- `n`: Create new issue
- `c`: Close issue/PR
- `R`: Reopen issue/PR
- `a`: Assign to yourself
- `l`: Add/remove labels
- `m`: Set milestone
- `d`: Add/delete comment
- `v`: Toggle vote/reaction

## Viewing Details

When you select an issue or pull request, press `Enter` or `Tab` to expand the right panel and view detailed information including:

- Full description
- Comments
- Labels
- Milestone
- Assignees
- Reaction counts

## Keyboard Shortcuts Reference

For a complete list of keyboard shortcuts, see the [Keybindings](keybindings.md) page.

## Tips and Tricks

- Use fuzzy matching in filters to quickly find items
- Press `?` at any time to show the help overlay
- The application automatically caches data for offline viewing
- Customize refresh intervals in the configuration file
- Use environment variables for sensitive data like tokens