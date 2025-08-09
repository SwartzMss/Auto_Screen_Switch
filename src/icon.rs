// 生成一个16x16 RGBA 的托盘图标（蓝色边框 + 白色填充）
pub fn generate_icon_rgba() -> Vec<u8> {
    let width = 16usize;
    let height = 16usize;
    let mut data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            let is_border = (x == 4 && (4..=11).contains(&y))
                || (x == 11 && (4..=11).contains(&y))
                || (y == 4 && (4..=11).contains(&x))
                || (y == 11 && (4..=11).contains(&x));

            if is_border {
                // 蓝色边框 (0, 120, 215)
                data[i] = 0;
                data[i + 1] = 120;
                data[i + 2] = 215;
                data[i + 3] = 255;
            } else if (5..=10).contains(&x) && (5..=10).contains(&y) {
                // 白色填充
                data[i] = 255;
                data[i + 1] = 255;
                data[i + 2] = 255;
                data[i + 3] = 255;
            } else if (y >= 13 && (6..=9).contains(&x)) || (y == 12 && (5..=10).contains(&x)) {
                // 简单的底座
                data[i] = 0;
                data[i + 1] = 120;
                data[i + 2] = 215;
                data[i + 3] = 255;
            } else {
                // 透明背景
                data[i + 3] = 0;
            }
        }
    }

    data
}
