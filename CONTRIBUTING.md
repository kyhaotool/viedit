# Đóng góp cho VieEdit

Cảm ơn bạn đã quan tâm! Dưới đây là vài nguyên tắc để mọi thứ đi nhanh.

## Quy trình

1. Mở issue mô tả vấn đề hoặc tính năng trước khi bắt tay vào việc lớn.
2. Fork, tạo nhánh: `feat/ten-tinh-nang` hoặc `fix/ten-loi`.
3. Chạy `cargo fmt` và `cargo clippy -- -D warnings`.
4. Chạy `cargo test` — test xuất video phải xanh.
5. Mở pull request, mô tả rõ *vì sao* thay đổi.

## Nguyên tắc mã nguồn

- **Giao diện phải bằng tiếng Việt.** Chuỗi hiển thị mới phải được thêm vào
  `src/i18n.rs` cho cả `VI` và `EN`.
- **Không thêm phụ thuộc nặng** nếu có thể tự viết dưới 100 dòng.
- **Không chặn luồng giao diện**: mọi việc tốn thời gian (giải mã, gọi FFmpeg)
  phải chạy trong thread riêng và gửi kết quả về qua kênh `mpsc`.
- **Model tách khỏi UI**: thay đổi dữ liệu nằm trong `model.rs`, không nằm
  trong widget.
- Mọi thao tác làm thay đổi dự án phải gọi `checkpoint()` trước khi sửa để
  hoàn tác được.

## Định dạng commit

```
feat: thêm hiệu ứng chuyển cảnh mờ dần
fix: không xuất được khi dòng thời gian chỉ có lớp phủ
docs: bổ sung hướng dẫn cài trên Fedora
```

## Báo lỗi

Kèm theo:

- Hệ điều hành và phiên bản
- Phiên bản FFmpeg (`ffmpeg -version`)
- Các bước tái hiện
- Ảnh chụp màn hình nếu có

## Giấy phép đóng góp

Khi gửi pull request, bạn đồng ý phát hành phần đóng góp của mình theo giấy
phép MIT của dự án.
