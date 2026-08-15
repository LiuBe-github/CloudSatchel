//! 主机性能监控采样
//!
//! 设计目标：
//! - 所有数据只在本地采集，不联网、不写注册表、不需要管理员权限；
//! - 采样与计算放在独立后台线程，1 秒刷新一次，不阻塞 Tauri 主线程；
//! - 前端按需调用 `get_perf_snapshot` 拉取最近一帧，不在后台累积事件队列；
//! - 开关关闭时立即停止更新快照，采集线程进入低开销等待。

#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

// ---------------------------------------------------------------------------
// 对外数据结构（与前端 camelCase 对应）
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSnapshot {
    pub timestamp: u64,
    pub cpu: CpuMetrics,
    pub gpu: GpuMetrics,
    pub memory: MemoryMetrics,
    pub network: NetworkMetrics,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuMetrics {
    /// 整机 CPU 总占用率（0-100）
    pub usage: f32,
    /// CPU 温度（可用时）
    pub temperature: Option<f32>,
    /// 当前频率 MHz（可用时）
    pub current_frequency_mhz: Option<f64>,
    /// 基准/最大频率 MHz（可用时）
    pub base_frequency_mhz: Option<f64>,
    /// 物理核心数（可用时）
    pub core_count: Option<usize>,
    /// 逻辑处理器数
    pub logical_processor_count: usize,
    /// 当前进程数
    pub process_count: usize,
    /// 当前线程数
    pub thread_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetrics {
    pub name: Option<String>,
    pub utilization: Option<f32>,
    pub temperature: Option<f32>,
    pub memory_used_mb: Option<f64>,
    pub memory_total_mb: Option<f64>,
    pub shared_memory_used_mb: Option<f64>,
    pub driver_version: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    /// 物理内存占用率（0-100）
    pub usage: f32,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub pagefile_used_bytes: Option<u64>,
    pub pagefile_total_bytes: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMetrics {
    /// 上行速率 bytes/s
    pub upload_bytes_per_sec: f64,
    /// 下行速率 bytes/s
    pub download_bytes_per_sec: f64,
    pub adapter_name: Option<String>,
    pub link_speed_mbps: Option<u64>,
}

// ---------------------------------------------------------------------------
// 后台采集线程
// ---------------------------------------------------------------------------

static PERF_ENABLED: AtomicBool = AtomicBool::new(false);
static PERF_LATEST: Mutex<Option<PerfSnapshot>> = Mutex::new(None);
static PERF_THREAD: OnceLock<()> = OnceLock::new();

pub fn start() {
    ensure_running();
    PERF_ENABLED.store(true, Ordering::SeqCst);
}

pub fn stop() {
    PERF_ENABLED.store(false, Ordering::SeqCst);
    *PERF_LATEST.lock().unwrap() = None;
}

pub fn latest() -> Option<PerfSnapshot> {
    PERF_LATEST.lock().unwrap().clone()
}

fn ensure_running() {
    PERF_THREAD.get_or_init(|| {
        std::thread::spawn(|| {
            let mut sampler = Sampler::new();
            loop {
                if !PERF_ENABLED.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }

                let started = Instant::now();
                let snapshot = sampler.sample();
                *PERF_LATEST.lock().unwrap() = Some(snapshot);

                let elapsed = started.elapsed();
                if elapsed < Duration::from_secs(1) {
                    std::thread::sleep(Duration::from_secs(1) - elapsed);
                }
            }
        });
    });
}

struct Sampler {
    prev_idle: u64,
    prev_kernel: u64,
    prev_user: u64,
    prev_net_in: u64,
    prev_net_out: u64,
    prev_net_at: Option<Instant>,
    physical_cores: Option<usize>,
    nvml: Option<nvml_wrapper::Nvml>,
}

impl Sampler {
    fn new() -> Self {
        let physical_cores = sysinfo::System::new_all().physical_core_count();
        let nvml = nvml_wrapper::Nvml::init().ok();
        Self {
            prev_idle: 0,
            prev_kernel: 0,
            prev_user: 0,
            prev_net_in: 0,
            prev_net_out: 0,
            prev_net_at: None,
            physical_cores,
            nvml,
        }
    }

    fn sample(&mut self) -> PerfSnapshot {
        let cpu = self.sample_cpu();
        let memory = sample_memory();
        let network = self.sample_network();
        let gpu = self.sample_gpu();

        PerfSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            cpu,
            gpu,
            memory,
            network,
        }
    }

    fn sample_cpu(&mut self) -> CpuMetrics {
        let mut idle = windows_sys::Win32::Foundation::FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = windows_sys::Win32::Foundation::FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut user = windows_sys::Win32::Foundation::FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        unsafe {
            let _ = windows_sys::Win32::System::Threading::GetSystemTimes(
                &mut idle,
                &mut kernel,
                &mut user,
            );
        }
        let idle = filetime_u64(idle);
        let kernel = filetime_u64(kernel);
        let user = filetime_u64(user);

        let mut usage = 0.0f32;
        let total = kernel.saturating_sub(self.prev_kernel)
            + user.saturating_sub(self.prev_user);
        let idle_delta = idle.saturating_sub(self.prev_idle);
        if total > 0 {
            let busy = total.saturating_sub(idle_delta);
            usage = (busy as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
        }
        self.prev_idle = idle;
        self.prev_kernel = kernel;
        self.prev_user = user;

        let (current_mhz, base_mhz) = cpu_frequencies();
        let logical_processor_count = unsafe {
            windows_sys::Win32::System::Threading::GetActiveProcessorCount(u16::MAX) as usize
        };
        let (process_count, thread_count) = process_thread_counts();

        CpuMetrics {
            usage,
            temperature: cpu_temperature(),
            current_frequency_mhz: current_mhz,
            base_frequency_mhz: base_mhz,
            core_count: self.physical_cores,
            logical_processor_count,
            process_count,
            thread_count,
        }
    }

    fn sample_network(&mut self) -> NetworkMetrics {
        let counters = network_counters();
        let now = Instant::now();
        let (upload, download) = if let Some(prev_at) = self.prev_net_at {
            let dt = now.duration_since(prev_at).as_secs_f64();
            if dt > 0.0 {
                let up_delta = counter_delta(counters.0, self.prev_net_in);
                let down_delta = counter_delta(counters.1, self.prev_net_out);
                (up_delta / dt, down_delta / dt)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };
        self.prev_net_in = counters.0;
        self.prev_net_out = counters.1;
        self.prev_net_at = Some(now);

        NetworkMetrics {
            upload_bytes_per_sec: upload,
            download_bytes_per_sec: download,
            adapter_name: counters.2,
            link_speed_mbps: counters.3,
        }
    }

    fn sample_gpu(&self) -> GpuMetrics {
        let Some(nvml) = self.nvml.as_ref() else {
            return GpuMetrics::default();
        };
        let Ok(device_count) = nvml.device_count() else {
            return GpuMetrics::default();
        };
        if device_count == 0 {
            return GpuMetrics::default();
        }
        let Ok(device) = nvml.device_by_index(0) else {
            return GpuMetrics::default();
        };

        let name = device.name().ok();
        let utilization = device
            .utilization_rates()
            .ok()
            .map(|u| u.gpu as f32);
        let temperature = device
            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
            .ok()
            .map(|t| t as f32);
        let memory_used_mb = device
            .memory_info()
            .ok()
            .map(|m| (m.used as f64) / 1024.0 / 1024.0);
        let memory_total_mb = device
            .memory_info()
            .ok()
            .map(|m| (m.total as f64) / 1024.0 / 1024.0);
        let driver_version = nvml.sys_driver_version().ok();

        GpuMetrics {
            name,
            utilization,
            temperature,
            memory_used_mb,
            memory_total_mb,
            shared_memory_used_mb: None,
            driver_version,
        }
    }
}

// ---------------------------------------------------------------------------
// 底层 Windows API / sysinfo 辅助函数
// ---------------------------------------------------------------------------

fn cpu_temperature() -> Option<f32> {
    let components = sysinfo::Components::new_with_refreshed_list();
    let mut fallback: Option<f32> = None;
    let mut matched: Option<f32> = None;
    for component in &components {
        let temp = component.temperature();
        if !temp.is_finite() || temp <= 0.0 {
            continue;
        }
        let label = component.label().to_ascii_lowercase();
        if label.contains("cpu")
            || label.contains("core")
            || label.contains("package")
            || label.contains("tctl")
            || label.contains("ccd")
            || label.contains("processor")
        {
            matched = Some(match matched {
                Some(v) => v.max(temp),
                None => temp,
            });
        } else {
            fallback = Some(match fallback {
                Some(v) => v.max(temp),
                None => temp,
            });
        }
    }
    matched.or(fallback)
}

fn cpu_frequencies() -> (Option<f64>, Option<f64>) {
    use windows_sys::Win32::System::Power::{CallNtPowerInformation, ProcessorInformation};
    let count = unsafe { windows_sys::Win32::System::Threading::GetActiveProcessorCount(u16::MAX) };
    if count == 0 {
        return (None, None);
    }
    let mut infos =
        vec![
            windows_sys::Win32::System::Power::PROCESSOR_POWER_INFORMATION {
                Number: 0,
                MaxMhz: 0,
                CurrentMhz: 0,
                MhzLimit: 0,
                MaxIdleState: 0,
                CurrentIdleState: 0,
            };
            count as usize
        ];
    let ok = unsafe {
        CallNtPowerInformation(
            ProcessorInformation,
            std::ptr::null(),
            0,
            infos.as_mut_ptr() as *mut core::ffi::c_void,
            (infos.len() * std::mem::size_of::<windows_sys::Win32::System::Power::PROCESSOR_POWER_INFORMATION>())
                as u32,
        )
    };
    if ok != 0 {
        return (None, None);
    }

    let current_sum: u64 = infos.iter().map(|i| i.CurrentMhz as u64).sum();
    let base_sum: u64 = infos.iter().map(|i| i.MaxMhz as u64).sum();
    let valid_current = infos.iter().filter(|i| i.CurrentMhz > 0).count();
    let valid_base = infos.iter().filter(|i| i.MaxMhz > 0).count();
    let current = if valid_current > 0 {
        Some(current_sum as f64 / valid_current as f64)
    } else {
        None
    };
    let base = if valid_base > 0 {
        Some(base_sum as f64 / valid_base as f64)
    } else {
        None
    };
    (current, base)
}

fn filetime_u64(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

fn process_thread_counts() -> (usize, usize) {
    use std::mem::size_of;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return (0, 0);
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        };

        let mut process_count = 0usize;
        let mut thread_count = 0usize;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                process_count += 1;
                thread_count += entry.cntThreads as usize;
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        windows_sys::Win32::Foundation::CloseHandle(snapshot);
        (process_count, thread_count)
    }
}

fn sample_memory() -> MemoryMetrics {
    use std::mem::size_of;
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };
        if GlobalMemoryStatusEx(&mut status) == 0 {
            return MemoryMetrics::default();
        }
        let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
        MemoryMetrics {
            usage: status.dwMemoryLoad as f32,
            used_bytes: used,
            available_bytes: status.ullAvailPhys,
            total_bytes: status.ullTotalPhys,
            pagefile_used_bytes: Some(status.ullTotalPageFile.saturating_sub(status.ullAvailPageFile)),
            pagefile_total_bytes: Some(status.ullTotalPageFile),
        }
    }
}

/// 返回 (in_octets, out_octets, adapter_name, link_speed_mbps)
fn network_counters() -> (u64, u64, Option<String>, Option<u64>) {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetIfTable, MIB_IFTABLE, IF_OPER_STATUS_OPERATIONAL, IF_TYPE_SOFTWARE_LOOPBACK,
    };

    let mut size: u32 = 0;
    unsafe {
        // 第一次调用取得所需缓冲区大小
        let _ = GetIfTable(std::ptr::null_mut(), &mut size, 0);
        if size == 0 {
            return (0, 0, None, None);
        }
        let mut buf = vec![0u8; size as usize];
        let table = buf.as_mut_ptr() as *mut MIB_IFTABLE;
        if GetIfTable(table, &mut size, 0) != 0 {
            return (0, 0, None, None);
        }

        let count = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), count);
        let mut total_in: u64 = 0;
        let mut total_out: u64 = 0;
        let mut adapter_name: Option<String> = None;
        let mut link_speed_mbps: Option<u64> = None;
        let mut best_activity = 0u64;

        for row in rows {
            let up = row.dwOperStatus == IF_OPER_STATUS_OPERATIONAL;
            let loopback = row.dwType == IF_TYPE_SOFTWARE_LOOPBACK;
            if !up || loopback {
                continue;
            }

            total_in = total_in.wrapping_add(row.dwInOctets as u64);
            total_out = total_out.wrapping_add(row.dwOutOctets as u64);

            let activity = row.dwInOctets as u64 + row.dwOutOctets as u64;
            if activity > best_activity {
                best_activity = activity;
                adapter_name = Some(ansi_name(&row.bDescr));
                if row.dwSpeed > 0 {
                    // dwSpeed 单位为 bit/s
                    link_speed_mbps = Some((row.dwSpeed as u64 / 1_000_000).max(1));
                }
            }
        }
        (total_in, total_out, adapter_name, link_speed_mbps)
    }
}

fn ansi_name(buf: &[u8]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// 处理 32 位计数器回绕：本机网络计数器按 u32 存储，采样间隔 1s，正常不会回绕。
fn counter_delta(current: u64, previous: u64) -> f64 {
    if current >= previous {
        (current - previous) as f64
    } else {
        (current.wrapping_add(1u64 << 32).wrapping_sub(previous)) as f64
    }
}
