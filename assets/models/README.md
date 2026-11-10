# Mô hình nhận dạng giọng nói

Thư mục này chứa mô hình **Whisper** (định dạng `ggml-*.bin`) dùng cho tính năng
**Phụ đề → Phụ đề tự động**.

## Cách lấy mô hình

Cách 1 – trong ứng dụng:

1. Mở **Phụ đề → Phụ đề tự động…**
2. Nhấn **Tải mô hình** (khoảng 141 MB, chỉ tải một lần)
3. Chọn ngôn ngữ rồi nhấn **Bắt đầu nhận dạng**

Cách 2 – tải thủ công rồi chép vào thư mục này:

| Mô hình | Dung lượng | Chất lượng |
|---|---|---|
| `ggml-tiny.bin` | 75 MB | Nhanh nhất, độ chính xác thấp |
| `ggml-base.bin` | 141 MB | Cân bằng (mặc định) |
| `ggml-small.bin` | 466 MB | Chính xác hơn, chậm hơn |

```bash
curl -L -o ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

Ứng dụng tự tìm mô hình trong các thư mục sau (theo thứ tự):

1. `<thư mục chứa viedit.exe>/models`
2. `<thư mục chứa viedit.exe>/assets/models`
3. `assets/models` (khi chạy từ mã nguồn)

Việc nhận dạng chạy hoàn toàn trên máy, không gửi dữ liệu lên mạng.
