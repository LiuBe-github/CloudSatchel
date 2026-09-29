//! CPU 温度数据源链（不提权、不装驱动、不写注册表）
//!
//! Windows 没有给桌面应用提供读 CPU 温度的公开用户态接口（温度在 MSR/SMN 等
//! ring-0 资源里），因此本模块只做「合法的旁路读取」，按优先级依次尝试：
//!
//! 1. **PDH 热区性能计数器** `\Thermal Zone Information(*)\Temperature`
//!    （系统自带、普通权限可读；固件不暴露热区的机器——如部分联想笔记本——直接失败）；
//! 2. **HWiNFO64 共享内存** `Global\HWiNFO_SENS_SM2`：用户自行安装 HWiNFO64 并开启
//!    Shared Memory Support 后，传感器数据写入内存映射文件，本进程普通权限只读映射
//!    解析（Rainmeter / RTSS 等同款官方公开接口）。
//!
//! 全部失败返回 `None`：任务栏小组件显示 `--`，主面板显示「不可用」。

#![allow(non_snake_case)]

/// 读取当前 CPU 温度（°C）。依次尝试各数据源，全部失败返回 None。
pub fn read() -> Option<f32> {
    pdh_thermal_zone().or_else(hwinfo_sm2)
}

// ---------------------------------------------------------------------------
// 数据源 1：PDH \Thermal Zone Information(*)\Temperature（ACPI 热区）
// ---------------------------------------------------------------------------

/// 计数器原始值 → 摄氏度。标准单位为开尔文；个别系统按 1/10 开尔文上报。
/// 超出合理范围（0~150°C）一律视为无效数据。
pub(crate) fn kelvin_to_celsius(raw: f64) -> Option<f32> {
    if !raw.is_finite() || raw <= 0.0 {
        return None;
    }
    let kelvin = if raw > 1000.0 { raw / 10.0 } else { raw };
    let celsius = kelvin - 273.15;
    if celsius.is_finite() && celsius > 0.0 && celsius < 150.0 {
        Some(celsius as f32)
    } else {
        None
    }
}

fn pdh_thermal_zone() -> Option<f32> {
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        PDH_MORE_DATA,
    };
    unsafe {
        let mut query: isize = 0;
        if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
            return None;
        }
        let result = (|| {
            let mut counter: isize = 0;
            // 英文计数器路径与系统区域设置无关
            if PdhAddEnglishCounterW(
                query,
                windows_sys::core::w!("\\Thermal Zone Information(*)\\Temperature"),
                0,
                &mut counter,
            ) != 0
            {
                return None;
            }
            // 热区温度是瞬时值计数器，一次采集即可
            if PdhCollectQueryData(query) != 0 {
                return None;
            }
            let mut size: u32 = 0;
            let mut count: u32 = 0;
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            );
            if status != PDH_MORE_DATA || size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
            );
            if status != 0 || count == 0 {
                return None;
            }
            let items = std::slice::from_raw_parts(
                buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                count as usize,
            );
            // 多个热区取最高值（最接近 CPU 负载热度）
            let mut best: Option<f32> = None;
            for item in items {
                if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA {
                    continue;
                }
                if let Some(c) = kelvin_to_celsius(item.FmtValue.Anonymous.doubleValue) {
                    best = Some(best.map_or(c, |b: f32| b.max(c)));
                }
            }
            best
        })();
        let _ = PdhCloseQuery(query);
        result
    }
}

// ---------------------------------------------------------------------------
// 数据源 2：HWiNFO64 共享内存（Global\HWiNFO_SENS_SM2）
// ---------------------------------------------------------------------------

/// 布局常量：按 namazso 对真实共享内存的实测（pack(1)）。
/// 官方 SDK 头若按默认对齐声明，value 会在 0x120，但实际内存布局为 packed，恒取 0x11C。
const SM2_MAGIC_LE: u32 = 0x48576953; // 'SiWH'（小端读取）
const SM2_MAGIC_BE: u32 = 0x53695748; // 逆序兼容
const SM2_TYPE_TEMPERATURE: u32 = 1;
const SM2_HEADER_LEN: usize = 44;
/// 元素字段偏移（packed 布局）
const SM2_ENTRY_SENSOR_INDEX: usize = 0x04;
const SM2_ENTRY_LABEL_ORIG: usize = 0x0C;
const SM2_ENTRY_LABEL_USER: usize = 0x8C;
const SM2_ENTRY_UNIT: usize = 0x10C;
const SM2_ENTRY_VALUE_PACKED: usize = 0x11C; // value（f64）偏移
const SM2_SENSOR_NAME_ORIG: usize = 0x08;
const SM2_SENSOR_NAME_USER: usize = 0x88;
/// 头部字段合法性上限：防御撕裂/损坏头部导致越界读
const SM2_MAX_ELEMENTS: u32 = 4096;
const SM2_MAX_TOTAL: usize = 4 * 1024 * 1024;

fn hwinfo_sm2() -> Option<f32> {
    use windows::core::w;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ,
    };
    unsafe {
        let handle = OpenFileMappingW(FILE_MAP_READ.0, false, w!("Global\\HWiNFO_SENS_SM2")).ok()?;
        let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
        let ptr = view.Value as *const u8;
        let result = if ptr.is_null() {
            None
        } else {
            let header = std::slice::from_raw_parts(ptr, SM2_HEADER_LEN);
            let total = sm2_total_span(header);
            let parsed = match total {
                Some(total) => {
                    let buf = std::slice::from_raw_parts(ptr, total);
                    parse_sm2(buf)
                }
                None => None,
            };
            let _ = UnmapViewOfFile(view);
            parsed
        };
        let _ = CloseHandle(handle);
        result
    }
}

fn u32le(buf: &[u8], offset: usize) -> Option<u32> {
    buf.get(offset..offset + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

/// 从头部计算映射区总跨度；头部字段超界/非法 → None（不读数据区）
fn sm2_total_span(header: &[u8]) -> Option<usize> {
    if header.len() < SM2_HEADER_LEN {
        return None;
    }
    let magic = u32le(header, 0)?;
    if magic != SM2_MAGIC_LE && magic != SM2_MAGIC_BE {
        return None;
    }
    let sensor_off = u32le(header, 0x14)? as usize;
    let sensor_size = u32le(header, 0x18)? as usize;
    let sensor_count = u32le(header, 0x1C)?;
    let entry_off = u32le(header, 0x20)? as usize;
    let entry_size = u32le(header, 0x24)? as usize;
    let entry_count = u32le(header, 0x28)?;
    // 元素尺寸下限：需能容纳到 value 字段结束（0x124 / 0x128）
    if entry_count == 0
        || entry_count > SM2_MAX_ELEMENTS
        || sensor_count > SM2_MAX_ELEMENTS
        || entry_size < SM2_ENTRY_VALUE_PACKED + 8
        || entry_size > 4096
        || sensor_size < SM2_SENSOR_NAME_USER + 128
        || sensor_size > 4096
        || sensor_off < SM2_HEADER_LEN
        || entry_off < SM2_HEADER_LEN
    {
        return None;
    }
    let entry_end = entry_off.checked_add(entry_size.checked_mul(entry_count as usize)?)?;
    let sensor_end = sensor_off.checked_add(sensor_size.checked_mul(sensor_count as usize)?)?;
    let total = entry_end.max(sensor_end);
    if total > SM2_MAX_TOTAL {
        return None;
    }
    Some(total)
}

/// 标签优先级（越小越优先）：Tctl/Tdie（AMD）→ CPU Package（Intel）
/// → 含 Package 的 CPU 传感器 → CPU 传感器上的 CPU 标签
pub(crate) fn cpu_label_priority(label: &str, sensor: &str) -> Option<u8> {
    let label = label.to_ascii_lowercase();
    let sensor = sensor.to_ascii_lowercase();
    if label.contains("tctl") || label.contains("tdie") {
        return Some(0);
    }
    if label.contains("cpu package") || label == "package" {
        return Some(1);
    }
    if label.contains("package") && sensor.contains("cpu") {
        return Some(2);
    }
    if sensor.contains("cpu") && label.contains("cpu") && !label.contains("core") {
        return Some(3);
    }
    None
}

/// 解析 SM2 内存映像，返回 CPU 温度（°C）。纯函数，便于单测。
pub(crate) fn parse_sm2(buf: &[u8]) -> Option<f32> {
    let total = sm2_total_span(buf)?;
    if buf.len() < total {
        return None;
    }
    let sensor_off = u32le(buf, 0x14)? as usize;
    let sensor_size = u32le(buf, 0x18)? as usize;
    let entry_off = u32le(buf, 0x20)? as usize;
    let entry_size = u32le(buf, 0x24)? as usize;
    let entry_count = u32le(buf, 0x28)? as usize;
    // value 固定取 packed 布局偏移（namazso 对真实内存的实测；官方头若按默认对齐
    // 声明会是 0x120，但实际共享内存按 pack(1) 布局，0x11C 处才是 value）
    let value_offset = SM2_ENTRY_VALUE_PACKED;

    let cstr = |offset: usize, len: usize| -> String {
        let Some(field) = buf.get(offset..offset + len) else {
            return String::new();
        };
        let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
        String::from_utf8_lossy(&field[..end]).trim().to_string()
    };
    let sensor_name = |index: usize| -> String {
        let base = sensor_off + index.saturating_mul(sensor_size);
        let user = cstr(base + SM2_SENSOR_NAME_USER, 128);
        if !user.is_empty() {
            return user;
        }
        cstr(base + SM2_SENSOR_NAME_ORIG, 128)
    };

    let mut best: Option<(u8, f32)> = None;
    for i in 0..entry_count {
        let base = entry_off + i * entry_size;
        if u32le(buf, base)? != SM2_TYPE_TEMPERATURE {
            continue;
        }
        let unit = cstr(base + SM2_ENTRY_UNIT, 16);
        if !unit.contains('C') {
            continue; // 只接受摄氏度读数（HWiNFO 温度单位恒为 °C）
        }
        let label_user = cstr(base + SM2_ENTRY_LABEL_USER, 128);
        let label = if !label_user.is_empty() {
            label_user
        } else {
            cstr(base + SM2_ENTRY_LABEL_ORIG, 128)
        };
        let sensor_idx = u32le(buf, base + SM2_ENTRY_SENSOR_INDEX)? as usize;
        let sensor = sensor_name(sensor_idx);
        let Some(priority) = cpu_label_priority(&label, &sensor) else {
            continue;
        };
        let raw = buf
            .get(base + value_offset..base + value_offset + 8)
            .map(|s| f64::from_le_bytes(s.try_into().unwrap()))?;
        if !raw.is_finite() || raw <= 0.0 || raw >= 150.0 {
            continue;
        }
        let value = raw as f32;
        best = Some(match best {
            Some((p, _)) if p <= priority => (p, best.unwrap().1),
            _ => (priority, value),
        });
    }
    best.map(|(_, v)| v)
}

// ---------------------------------------------------------------------------
// 单元测试（纯逻辑部分；PDH / 共享内存属系统交互，靠实机验证）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kelvin_conversion_handles_standard_and_tenths() {
        // 273.15K = 0°C，按无效数据处理（传感器坏值哨兵）
        assert_eq!(kelvin_to_celsius(273.15), None);
        assert!((kelvin_to_celsius(274.15).unwrap() - 1.0).abs() < 0.01);
        assert!((kelvin_to_celsius(328.15).unwrap() - 55.0).abs() < 0.01);
        // 1/10 开尔文上报（3012 → 28.05°C）
        assert!((kelvin_to_celsius(3015.65).unwrap() - 28.415).abs() < 0.01);
        // 非法值
        assert_eq!(kelvin_to_celsius(0.0), None);
        assert_eq!(kelvin_to_celsius(f64::NAN), None);
        assert_eq!(kelvin_to_celsius(-5.0), None);
        // 超出合理范围（500K = 226.85°C → 拒绝）
        assert_eq!(kelvin_to_celsius(500.0), None);
    }

    #[test]
    fn label_priority_prefers_tctl_then_package() {
        assert_eq!(cpu_label_priority("CPU (Tctl/Tdie)", "CPU [#0]: AMD Ryzen 7 5800H"), Some(0));
        assert_eq!(cpu_label_priority("CPU Package", "CPU [#0]: Intel Core i9"), Some(1));
        assert_eq!(cpu_label_priority("Package", "CPU [#0]"), Some(1));
        assert_eq!(cpu_label_priority("CPU", "CPU [#0]: AMD Ryzen"), Some(3));
        // 每核温度不作为整机温度
        assert_eq!(cpu_label_priority("CPU Core #1", "CPU [#0]"), None);
        // GPU 传感器上的温度不匹配
        assert_eq!(cpu_label_priority("GPU Core", "GPU [#1]: NVIDIA"), None);
    }

    /// 构造一段合法的 SM2 映像（packed 布局）：1 个传感器 + 若干读数
    fn fake_sm2_packed(readings: &[(u32, u32, &str, &str, f64)]) -> Vec<u8> {
        let sensor_name = "CPU [#0]: AMD Ryzen 7 5800H: Enhanced";
        let sensor_off = SM2_HEADER_LEN;
        let sensor_size = 264usize;
        let entry_off = sensor_off + sensor_size;
        let entry_size = 316usize;
        let total = entry_off + entry_size * readings.len();
        let mut buf = vec![0u8; total];
        let put_u32 = |buf: &mut Vec<u8>, o: usize, v: u32| buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put_u32(&mut buf, 0x00, SM2_MAGIC_LE);
        put_u32(&mut buf, 0x04, 1);
        put_u32(&mut buf, 0x08, 1);
        put_u32(&mut buf, 0x14, sensor_off as u32);
        put_u32(&mut buf, 0x18, sensor_size as u32);
        put_u32(&mut buf, 0x1C, 1);
        put_u32(&mut buf, 0x20, entry_off as u32);
        put_u32(&mut buf, 0x24, entry_size as u32);
        put_u32(&mut buf, 0x28, readings.len() as u32);
        buf[sensor_off + SM2_SENSOR_NAME_ORIG..sensor_off + SM2_SENSOR_NAME_ORIG + sensor_name.len()]
            .copy_from_slice(sensor_name.as_bytes());
        for (i, (ty, sensor_idx, label, unit, value)) in readings.iter().enumerate() {
            let base = entry_off + i * entry_size;
            put_u32(&mut buf, base, *ty);
            put_u32(&mut buf, base + SM2_ENTRY_SENSOR_INDEX, *sensor_idx);
            buf[base + SM2_ENTRY_LABEL_ORIG..base + SM2_ENTRY_LABEL_ORIG + label.len()]
                .copy_from_slice(label.as_bytes());
            buf[base + SM2_ENTRY_UNIT..base + SM2_ENTRY_UNIT + unit.len()]
                .copy_from_slice(unit.as_bytes());
            buf[base + SM2_ENTRY_VALUE_PACKED..base + SM2_ENTRY_VALUE_PACKED + 8]
                .copy_from_slice(&value.to_le_bytes());
        }
        buf
    }

    #[test]
    fn sm2_parse_finds_tctl_temperature() {
        let buf = fake_sm2_packed(&[
            (7, 0, "CPU", "%", 21.0),                       // 占用率读数，跳过
            (1, 0, "CPU (Tctl/Tdie)", "°C", 55.5),          // 目标
            (1, 0, "CPU Core #1", "°C", 51.0),              // 每核温度，不匹配
        ]);
        assert_eq!(parse_sm2(&buf), Some(55.5));
    }

    #[test]
    fn sm2_parse_falls_back_to_package_label() {
        let buf = fake_sm2_packed(&[(1, 0, "CPU Package", "°C", 62.25)]);
        assert_eq!(parse_sm2(&buf), Some(62.25));
    }

    #[test]
    fn sm2_parse_rejects_bad_magic_and_garbage() {
        let mut buf = fake_sm2_packed(&[(1, 0, "CPU (Tctl/Tdie)", "°C", 55.0)]);
        buf[0] = 0xFF; // 破坏 magic
        assert_eq!(parse_sm2(&buf), None);
        assert_eq!(parse_sm2(&[0u8; 16]), None);
        // 非摄氏度单位被跳过
        let buf = fake_sm2_packed(&[(1, 0, "CPU (Tctl/Tdie)", "RPM", 55.0)]);
        assert_eq!(parse_sm2(&buf), None);
        // 无 CPU 相关标签
        let buf = fake_sm2_packed(&[(1, 0, "GPU Core", "°C", 66.0)]);
        assert_eq!(parse_sm2(&buf), None);
        // 温度值超出合理范围
        let buf = fake_sm2_packed(&[(1, 0, "CPU (Tctl/Tdie)", "°C", 250.0)]);
        assert_eq!(parse_sm2(&buf), None);
    }
}
