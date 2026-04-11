//! 水印数据打包与解析工具模块
//! 
//! 盲水印底层需要定长的 bit 流。本模块负责将用户的变长字符串
//! 封装为固定长度的字节数组 (目前定为 64 字节，即 512 bits)。
//! 数据结构：[1 Byte 真实长度] + [N Bytes 真实文本] + [剩余全填随机盐 (Padding)]

use rand::Rng;

/// 设定固定封装长度。
/// 64 字节对应 512 bit 的底层提取长度。如果需要更长文本，可适当调大 (如 128)。
pub const FIXED_PAYLOAD_SIZE: usize = 64;

/// 将明文字符串打包为定长字节流，并用随机数填充空白区域。
/// 
/// 随机加盐的好处：即使输入相同的水印，每次生成的底层特征也完全不同，防止被逆向分析。
pub fn pack_watermark(text: &str) -> Result<Vec<u8>, String> {
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();

    // 预留 1 个字节存储 Header (长度信息)
    if text_len > FIXED_PAYLOAD_SIZE - 1 {
        return Err(format!(
            "水印超长：最大允许 {} 字节，当前 {} 字节",
            FIXED_PAYLOAD_SIZE - 1,
            text_len
        ));
    }

    // 初始化定长缓冲区，默认全为 0
    let mut payload = vec![0u8; FIXED_PAYLOAD_SIZE];

    // 写入 Header：真实数据的长度
    payload[0] = text_len as u8;

    // 写入 Payload：真实的文本数据
    payload[1..=text_len].copy_from_slice(text_bytes);

    // 写入 Padding：使用随机盐填满剩余空间，避免频域出现规律性特征
    let mut rng = rand::thread_rng();
    // 这里使用 rng.fill() 进行批量填充
    let padding_slice = &mut payload[(text_len + 1)..FIXED_PAYLOAD_SIZE];
    rng.fill(padding_slice);

    Ok(payload)
}

/// 将提取出的定长字节流解包，去除随机盐，还原为明文字符串。
pub fn unpack_watermark(payload: &[u8]) -> Result<String, String> {
    // 防御性编程：确保送入的数据长度符合预期
    if payload.len() != FIXED_PAYLOAD_SIZE {
        return Err("数据长度异常，底层提取失败！".to_string());
    }

    // 1. 解析 Header 确定真实长度
    let text_len = payload[0] as usize;

    // 安全拦截：如果用户密码 (Seed) 输错，提取出来的首字节极率是乱码。
    // 如果长度指示器越界，直接判为提取失败，防止后续切片 panic。
    if text_len > FIXED_PAYLOAD_SIZE - 1 {
        return Err("解析失败：密码(Seed)错误，或图片内无有效水印！".to_string());
    }

    // 2. 截取有效数据并尝试转换为 UTF-8 字符串
    let text_bytes = &payload[1..=text_len];
    String::from_utf8(text_bytes.to_vec())
        .map_err(|_| "解析失败：密码(Seed)错误，或图片文件已损坏！".to_string())
}