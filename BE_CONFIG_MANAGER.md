# BE Configuration Manager Tool

This tool allows you to manage configuration variables for Backend (BE) nodes in a Doris cluster.

## Features

- List and select clusters
- View current configuration values across all BE nodes
- Update configuration values on all BE nodes simultaneously
- Generic configuration management (not limited to max_tablet_version_num)

## Usage

1. Run the cloud-cli tool
2. Select "BE" from the main menu
3. Select "be-config-manager" from the BE tools menu
4. Follow the prompts to:
   - Select a cluster
   - Enter the configuration name you want to manage
   - View current values across all nodes
   - Enter a new value to update all nodes

## Examples

Some common configuration variables you can manage:

- `max_tablet_version_num` - Maximum number of tablet versions
- `storage_min_columns_per_segment` - Minimum columns per segment
- `tablet_meta_checkpoint_interval_sec` - Tablet meta checkpoint interval
- `report_disk_state_interval_seconds` - Disk state reporting interval

## Notes

- The tool will attempt to update the configuration on all BE nodes in the selected cluster
- Configuration changes take effect immediately on supported variables
- Some configuration variables may require a BE node restart to take effect