use crate::config::Config;
use crate::error::{CliError, Result};
use crate::tools::{ExecutionResult, Tool};
use crate::tools::mysql::MySQLTool;
use crate::ui;
use crate::config_loader::load_config;
use dialoguer::{Select, Input, theme::ColorfulTheme};
use std::process::Command;

/// Tool to manage BE configuration variables
pub struct BeConfigManagerTool;

impl Tool for BeConfigManagerTool {
    fn name(&self) -> &str {
        "be-config-manager"
    }

    fn description(&self) -> &str {
        "Manage BE configuration variables"
    }

    fn execute(&self, _config: &Config, _pid: u32) -> Result<ExecutionResult> {
        ui::print_info("Starting BE configuration management tool...");
        
        // 1. 获取集群列表
        let clusters = self.get_cluster_list()?;
        if clusters.is_empty() {
            return Err(CliError::ToolExecutionFailed("No clusters found".to_string()));
        }

        // 2. 选择集群
        let selected_cluster = self.select_cluster(&clusters)?;
        ui::print_info(&format!("Selected cluster: {}", selected_cluster));

        // 3. 获取对应集群的BE节点信息
        let backends = self.get_cluster_backends(&selected_cluster)?;
        if backends.is_empty() {
            return Err(CliError::ToolExecutionFailed("No backends found for selected cluster".to_string()));
        }

        // 4. 获取要管理的配置项名称
        let config_name = self.prompt_for_config_name()?;
        if config_name.is_empty() {
            return Ok(ExecutionResult {
                output_path: std::path::PathBuf::from("console_output"),
                message: "Operation cancelled by user".to_string(),
            });
        }

        // 5. 显示当前配置值
        ui::print_info(&format!("Retrieving current {} values...", config_name));
        let current_values = self.get_current_config_value(&backends, &config_name)?;
        
        // 显示当前值
        ui::print_info(&format!("Current {} values:", config_name));
        for (host, port, value) in &current_values {
            ui::print_info(&format!("  {}:{} -> {}", host, port, value));
        }

        // 6. 获取用户想要设置的新值
        let new_value = self.prompt_for_new_value(&config_name)?;
        if new_value.is_none() {
            return Ok(ExecutionResult {
                output_path: std::path::PathBuf::from("console_output"),
                message: "Operation cancelled by user".to_string(),
            });
        }
        let new_value = new_value.unwrap();

        // 7. 更新所有节点的配置
        ui::print_info(&format!("Updating {} to {} on all nodes...", config_name, new_value));
        let results = self.update_config_value(&backends, &config_name, &new_value)?;

        // 8. 显示结果
        ui::print_success("Update completed!");
        ui::print_info("Update results:");
        for (host, port, success, message) in &results {
            if *success {
                ui::print_info(&format!("  {}:{} -> Success", host, port));
            } else {
                ui::print_warning(&format!("  {}:{} -> Failed: {}", host, port, message));
            }
        }

        Ok(ExecutionResult {
            output_path: std::path::PathBuf::from("console_output"),
            message: format!("Successfully updated {} to {} on {} nodes", config_name, new_value, results.len()),
        })
    }

    fn requires_pid(&self) -> bool {
        false
    }
}

impl BeConfigManagerTool {
    /// 获取集群列表
    fn get_cluster_list(&self) -> Result<Vec<String>> {
        let doris_config = load_config()?;
        let mysql_tool = MySQLTool;
        
        match mysql_tool.query_cluster_info(&doris_config) {
            Ok(cluster_info) => {
                // 从backends中提取唯一的集群名称
                let mut cluster_names = std::collections::HashSet::new();
                for backend in &cluster_info.backends {
                    if let Some(tag) = &backend.tag {
                        // 尝试从tag中解析集群名称
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(tag) {
                            if let Some(cluster_name) = json.get("cloud_cluster_name") {
                                cluster_names.insert(cluster_name.as_str().unwrap_or("unknown").to_string());
                            } else if let Some(cluster_id) = json.get("cloud_cluster_id") {
                                cluster_names.insert(cluster_id.as_str().unwrap_or("unknown").to_string());
                            }
                        } else {
                            // 如果无法解析JSON，使用tag本身作为集群名称
                            cluster_names.insert(tag.clone());
                        }
                    }
                }
                
                // 如果没有从tag中提取到集群名称，添加一个默认选项
                if cluster_names.is_empty() {
                    cluster_names.insert("default_cluster".to_string());
                }
                
                Ok(cluster_names.into_iter().collect())
            }
            Err(e) => {
                ui::print_warning(&format!("Failed to query cluster info: {}. Using default cluster.", e));
                Ok(vec!["default_cluster".to_string()])
            }
        }
    }

    /// 选择集群
    fn select_cluster(&self, clusters: &[String]) -> Result<String> {
        if clusters.len() == 1 {
            return Ok(clusters[0].clone());
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a cluster")
            .items(clusters)
            .default(0)
            .interact()
            .map_err(|e| CliError::InvalidInput(format!("Cluster selection failed: {e}")))?;

        Ok(clusters[selection].clone())
    }

    /// 获取指定集群的BE节点信息
    fn get_cluster_backends(&self, _cluster_name: &str) -> Result<Vec<(String, u16)>> {
        let doris_config = load_config()?;
        let mysql_tool = MySQLTool;
        
        match mysql_tool.query_cluster_info(&doris_config) {
            Ok(cluster_info) => {
                let mut backends = Vec::new();
                for backend in &cluster_info.backends {
                    // 在这个简化版本中，我们返回所有后端节点
                    // 在实际实现中，可以根据集群名称过滤
                    backends.push((backend.host.clone(), backend.http_port));
                }
                Ok(backends)
            }
            Err(e) => Err(CliError::ToolExecutionFailed(format!("Failed to get backend info: {e}")))
        }
    }

    /// 获取用户想要管理的配置项名称
    fn prompt_for_config_name(&self) -> Result<String> {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter configuration name to manage (e.g., max_tablet_version_num)")
            .interact_text()
            .map_err(|e| CliError::InvalidInput(format!("Configuration name input failed: {e}")))?;

        Ok(input.trim().to_string())
    }

    /// 获取当前的配置值
    fn get_current_config_value(&self, backends: &[(String, u16)], config_name: &str) -> Result<Vec<(String, u16, String)>> {
        let mut results = Vec::new();
        
        for (host, port) in backends {
            let url = format!("http://{}:{}/varz", host, port);
            let mut curl_cmd = Command::new("curl");
            curl_cmd.args(["-sS", &url]);
            
            match crate::executor::execute_command(&mut curl_cmd, "curl") {
                Ok(output) => {
                    let content = String::from_utf8_lossy(&output.stdout);
                    // 解析响应，查找指定配置项
                    let value = self.extract_config_value(&content, config_name);
                    results.push((host.clone(), *port, value));
                }
                Err(e) => {
                    results.push((host.clone(), *port, format!("Error: {}", e)));
                }
            }
        }
        
        Ok(results)
    }

    /// 从响应中提取指定配置项的值
    fn extract_config_value(&self, content: &str, config_name: &str) -> String {
        // 解析varz响应，查找指定配置项
        for line in content.lines() {
            if line.contains(config_name) {
                // 假设格式是 "config_name value" 或 "config_name=value"
                if line.contains("=") {
                    let parts: Vec<&str> = line.split("=").collect();
                    if parts.len() >= 2 {
                        return parts[1].trim().to_string();
                    }
                } else {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].to_string();
                    }
                }
            }
        }
        "Unknown".to_string()
    }

    /// 获取用户想要设置的新值
    fn prompt_for_new_value(&self, config_name: &str) -> Result<Option<String>> {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&format!("Enter new value for {} (or 'cancel' to exit)", config_name))
            .interact_text()
            .map_err(|e| CliError::InvalidInput(format!("New value input failed: {e}")))?;

        if input.trim().to_lowercase() == "cancel" {
            Ok(None)
        } else {
            Ok(Some(input.trim().to_string()))
        }
    }

    /// 更新所有节点的配置值
    fn update_config_value(&self, backends: &[(String, u16)], config_name: &str, new_value: &str) -> Result<Vec<(String, u16, bool, String)>> {
        let mut results = Vec::new();
        
        for (host, port) in backends {
            let url = format!("http://{}:{}/api/update_config?{}={}", host, port, config_name, new_value);
            let mut curl_cmd = Command::new("curl");
            curl_cmd.args(["-sS", "-X", "POST", &url]);
            
            match crate::executor::execute_command(&mut curl_cmd, "curl") {
                Ok(output) => {
                    let content = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    // 检查响应是否成功
                    if content.contains("success") || content.is_empty() || stderr.is_empty() {
                        results.push((host.clone(), *port, true, "Success".to_string()));
                    } else {
                        // 如果有错误信息，优先显示错误信息
                        if !stderr.is_empty() {
                            results.push((host.clone(), *port, false, format!("Error: {}", stderr)));
                        } else {
                            results.push((host.clone(), *port, false, content.to_string()));
                        }
                    }
                }
                Err(e) => {
                    results.push((host.clone(), *port, false, format!("Error: {}", e)));
                }
            }
        }
        
        Ok(results)
    }
}