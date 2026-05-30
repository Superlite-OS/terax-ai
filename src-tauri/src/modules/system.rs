use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub cpu: f64,
    pub mem: f64,
    pub user: String,
}

#[derive(Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub ip: String,
    pub protocol: String,
    pub pid: Option<u32>,
    pub process_name: String,
    pub state: String,
}

#[tauri::command]
pub fn get_system_processes() -> Result<Vec<ProcessInfo>, String> {
    let output = Command::new("ps")
        .args(["aux", "--sort=-pcpu"])
        .output()
        .map_err(|e| format!("ps failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            continue;
        }
        let user = parts[0].to_string();
        let pid: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let cpu: f64 = parts[2].parse().unwrap_or(0.0);
        let mem: f64 = parts[3].parse().unwrap_or(0.0);
        let command = parts[10..].join(" ");
        let name = std::path::Path::new(&command)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| command.clone());

        processes.push(ProcessInfo {
            name,
            pid,
            cpu,
            mem,
            user,
        });
    }

    Ok(processes)
}

#[tauri::command]
pub fn get_listening_ports() -> Result<Vec<PortInfo>, String> {
    let output = Command::new("ss")
        .args(["-tlnp"])
        .output()
        .map_err(|e| format!("ss failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let local_addr = parts[3];
        let state = parts[0].to_string();

        // Parse "ip:port" or "*:port"
        let (ip, port) = if let Some(colon_pos) = local_addr.rfind(':') {
            let ip = &local_addr[..colon_pos];
            let port_str = &local_addr[colon_pos + 1..];
            let port: u16 = match port_str.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            (ip.to_string(), port)
        } else {
            continue;
        };

        // Extract PID from "users:(("..."))
        let (pid, process_name) = if let Some(pid_start) = line.find("pid=") {
            let after_pid = &line[pid_start + 4..];
            if let Some(pid_end) = after_pid.find(',') {
                let pid_str = &after_pid[..pid_end];
                let pid: u32 = pid_str.parse().unwrap_or(0);
                let name = if let Some(name_start) = line.find("users:((\"") {
                    let name_part = &line[name_start + 9..];
                    if let Some(name_end) = name_part.find('"') {
                        name_part[..name_end].to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                (Some(pid), name)
            } else {
                (None, String::new())
            }
        } else {
            (None, String::new())
        };

        let protocol = if ip.contains(':') { "TCP6" } else { "TCP" }.to_string();

        ports.push(PortInfo {
            port,
            ip,
            protocol,
            pid,
            process_name,
            state,
        });
    }

    Ok(ports)
}

#[tauri::command]
pub fn kill_process(pid: u32) -> Result<bool, String> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("kill failed: {}", e))?;

    Ok(output.status.success())
}

#[tauri::command]
pub fn get_system_stats() -> Result<serde_json::Value, String> {
    // CPU usage from /proc/stat
    let cpu_line = std::fs::read_to_string("/proc/stat")
        .map_err(|e| format!("read /proc/stat: {}", e))?
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    let cpu_fields: Vec<u64> = cpu_line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();

    let cpu_percent = if cpu_fields.len() >= 4 {
        let total: u64 = cpu_fields.iter().sum();
        let idle = cpu_fields.get(3).copied().unwrap_or(0);
        let used = total - idle;
        (used as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };

    // Memory from /proc/meminfo
    let meminfo = std::fs::read_to_string("/proc/meminfo")
        .map_err(|e| format!("read /proc/meminfo: {}", e))?;

    let mut mem_total = 0u64;
    let mut mem_available = 0u64;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            mem_available = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    let mem_used_mb = ((mem_total - mem_available) / 1024) as u64;

    Ok(serde_json::json!({
        "cpu_percent": cpu_percent,
        "mem_total_mb": mem_total / 1024,
        "mem_used_mb": mem_used_mb,
    }))
}
