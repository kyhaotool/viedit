<p align="center">
  <img src="assets/icon.png" width="120" alt="VieEdit" />
</p>

<h1 align="center">VieEdit</h1>

<p align="center">
  Trình biên tập video <b>tiếng Việt</b>, viết hoàn toàn bằng <b>Rust</b>.<br />
  Chạy trên <b>Windows</b> và <b>Fedora Linux</b>. Mã nguồn mở, giấy phép MIT.
</p>

---

## VieEdit là gì?

VieEdit là một trình dựng phim gọn nhẹ, lấy cảm hứng từ các ứng dụng dựng video
phổ biến, nhưng **toàn bộ giao diện được Việt hoá** và **không phụ thuộc vào
trình duyệt hay Electron**.

Toàn bộ ứng dụng là một tệp thực thi Rust duy nhất, dùng [egui](https://github.com/emilk/egui)
để vẽ giao diện và [FFmpeg](https://ffmpeg.org) để giải mã / mã hoá video.

### Điểm khác biệt

| | VieEdit | Ứng dụng khác |
|---|---|---|
| Ngôn ngữ giao diện | Tiếng Việt là mặc định | Thường chỉ có tiếng Anh |
| Công nghệ | Rust + egui (native) | Electron / trình duyệt |
| Dung lượng | ~15 MB | 300 MB – 1 GB |
| Thư viện tiêu đề động | Đóng gói sẵn 158 hiệu ứng | Phải mua gói riêng |
| Giấy phép | MIT, mã nguồn mở | Đóng |

---

## Tính năng

- **Dòng thời gian nhiều lớp**: track Hình ảnh, Lớp phủ, Âm thanh.
- **Kéo – cắt – ghép trực tiếp**: kéo thân đoạn để di chuyển, kéo hai mép để
  cắt, nhấn `S` để cắt đôi tại con trỏ.
- **Bám dính thông minh**: đoạn phim tự bám vào con trỏ thời gian và các đoạn
  bên cạnh.
- **Thư viện tiêu đề động**: 158 tệp GIF/MP4 (tiêu đề, lower-third, nền chuyển
  sắc, hiệu ứng mạng xã hội, chuyển cảnh).
- **Xem trước theo thời gian thực**: giải mã GIF ngay trong tiến trình, giải mã
  video qua FFmpeg với bộ đệm khung hình.
- **Xuất video MP4**: bộ lọc `trim` / `concat` / `overlay` của FFmpeg, có thanh
  tiến trình.
- **Lưu / mở dự án**: định dạng `.vieedit` (JSON thuần, đọc được bằng tay).
- **Hoàn tác / làm lại** 80 bước.
- **Song ngữ**: Tiếng Việt ⇄ English ngay trong ứng dụng.
- **Nhớ cấu hình**: thư mục thư viện và ngôn ngữ được lưu giữa các lần chạy.
- **Không quảng cáo, không theo dõi, không cần mạng.**

---

## Cài đặt

### Tải bản dựng sẵn (nhanh nhất)

Vào trang [**Releases**](https://github.com/kyhaotool/viedit/releases), tải
`viedit-windows-x64.zip`, giải nén và chạy `viedit.exe`.
Nhớ cài FFmpeg (`winget install Gyan.FFmpeg`) để xem trước và xuất video.

### Yêu cầu chung

- [Rust](https://rustup.rs) ≥ 1.75
- [FFmpeg](https://ffmpeg.org/download.html) ≥ 6.0 (để xem trước và xuất video)

### Windows

```powershell
winget install Gyan.FFmpeg
winget install Rustlang.Rustup
```

Sau đó:

```powershell
git clone https://github.com/kyhaotool/viedit.git
cd viedit
cargo build --release
.\target\release\viedit.exe
```

> Nếu `cargo` báo thiếu linker, cài **Visual Studio Build Tools** với thành phần
> *Desktop development with C++*, hoặc dùng toolchain GNU:
> `rustup toolchain install stable-x86_64-pc-windows-gnu`.

### Fedora Linux

```bash
sudo dnf install ffmpeg ffmpeg-devel gcc gcc-c++ make \
    libxkbcommon-devel wayland-devel libX11-devel libXi-devel \
    mesa-libGL-devel mesa-libEGL-devel
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
git clone https://github.com/kyhaotool/viedit.git
cd viedit
cargo build --release
./target/release/viedit
```

Hoặc dùng script có sẵn:

```bash
./build.sh          # kiểm tra phụ thuộc, build và đóng gói vào ./dist
```

---

## Đóng gói bản phát hành

FFmpeg được gọi dưới dạng tiến trình con. VieEdit tự tìm `ffmpeg` theo thứ tự:

1. `ffmpeg/bin/ffmpeg` cạnh tệp thực thi
2. `bin/ffmpeg` cạnh tệp thực thi
3. `ffmpeg` trong `PATH`

Muốn phát hành bản "không cần cài FFmpeg", chỉ cần chép thư mục `ffmpeg/bin`
vào cạnh `viedit.exe`:

```
viedit/
├── viedit.exe
├── assets/
│   └── library/       ← thư viện tiêu đề động
└── ffmpeg/
    └── bin/
        ├── ffmpeg.exe
        └── ffprobe.exe
```

`build.ps1` (Windows) và `build.sh` (Fedora) tự tạo cấu trúc này trong `./dist`.

---

## Thư viện tài nguyên

Thư mục mặc định: `assets/library`, được quét đệ quy và phân loại theo tên thư mục:

| Từ khoá trong đường dẫn | Tab trong ứng dụng |
|---|---|
| `title`, `lower third` | Tiêu đề động |
| `transition` | Chuyển cảnh |
| `gradient` | Nền chuyển sắc |
| `social` | Mạng xã hội |

Bạn có thể trỏ VieEdit tới bất kỳ thư mục nào khác bằng nút **📁** trong panel
Thư viện — ví dụ một gói hiệu ứng bạn đã mua.

> **Lưu ý bản quyền:** các tệp trong `assets/library` chỉ là mẫu minh hoạ lấy từ
> một gói hiệu ứng bên thứ ba. Trước khi phát hành lại hoặc dùng thương mại, hãy
> kiểm tra giấy phép của gói gốc và thay bằng tài nguyên bạn có quyền sử dụng.

---

## Phím tắt

| Phím | Chức năng |
|---|---|
| `Space` | Phát / tạm dừng |
| `S` | Cắt đôi đoạn tại con trỏ |
| `Delete` | Xoá đoạn đang chọn |
| `Ctrl + S` | Lưu dự án |
| `Ctrl + E` | Xuất video |
| `Ctrl + Z` / `Ctrl + Y` | Hoàn tác / làm lại |
| `←` `→` | Dịch con trỏ 1 khung hình |
| `Home` / `End` | Về đầu / cuối |
| `Ctrl + lăn chuột` | Thu phóng dòng thời gian |
| `Shift + lăn chuột` | Cuộn ngang |

---

## Cấu trúc mã nguồn

```
src/
├── main.rs         Khởi động ứng dụng, biểu tượng cửa sổ
├── lib.rs          Gốc thư viện (để viết test tích hợp)
├── app.rs          Trạng thái và bố cục giao diện chính
├── timeline.rs     Widget dòng thời gian (vẽ + tương tác)
├── model.rs        Mô hình dự án: Project / Track / Clip / Source
├── media.rs        Cầu nối FFmpeg: probe, trích khung hình, xuất video
├── library.rs      Quét và phân loại thư viện tài nguyên
├── filepicker.rs   Hộp thoại chọn tệp thuần egui
├── i18n.rs         Toàn bộ chuỗi tiếng Việt / tiếng Anh
└── theme.rs        Bảng màu, font Be Vietnam Pro, kiểu dáng
tests/
└── export.rs       Test tích hợp: dựng phim → xuất MP4 thật
```

### Kiến trúc

```
      ┌──────────────┐   thao tác   ┌──────────────┐
      │   Giao diện  │ ───────────▶ │    Project   │
      │  (egui/eframe)│ ◀─────────── │  (model.rs)  │
      └──────┬───────┘   trạng thái  └──────┬───────┘
             │                              │
             │ yêu cầu khung hình           │ kế hoạch dựng phim
             ▼                              ▼
      ┌──────────────┐              ┌──────────────┐
      │ FrameLoader  │              │ filter_complex│
      │ (thread nền) │              │   (FFmpeg)    │
      └──────┬───────┘              └──────┬───────┘
             └──────────┬──────────────────┘
                        ▼
                   ffmpeg / ffprobe
```

Model hoàn toàn tách khỏi giao diện: mọi thao tác đều là biến đổi trên
`Project`, sau đó giao diện chỉ việc vẽ lại. Nhờ vậy hoàn tác/làm lại chỉ là
sao lưu và khôi phục `Project`.

---

## Kiểm thử

```bash
cargo test
```

`tests/export.rs` dựng một dự án nhỏ từ tài nguyên mẫu rồi **xuất ra MP4 thật**
bằng FFmpeg, sau đó kiểm tra tệp kết quả. Nếu máy không có FFmpeg, test tự bỏ qua.

### Bật CI trên GitHub

Quy trình build sẵn có ở `.github/workflows/build.yml.example` nhưng chưa được
kích hoạt (token đẩy mã nguồn chưa có scope `workflow`). Muốn bật:

```bash
gh auth refresh -s workflow
git mv .github/workflows/build.yml.example .github/workflows/build.yml
git commit -m "ci: bật workflow build" && git push
```

---

## Lộ trình

- [ ] Hiệu ứng chuyển cảnh giữa hai đoạn
- [ ] Track âm thanh riêng + trộn nhạc nền
- [ ] Phụ đề tự động từ tệp SRT
- [ ] Xuất theo mẻ (batch)
- [ ] Gói cài đặt `.msi` và `.rpm`

Đóng góp luôn được chào đón — xem [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Giấy phép

MIT © VieEdit contributors. Xem [LICENSE](LICENSE).

Font [Be Vietnam Pro](https://fonts.google.com/specimen/Be+Vietnam+Pro) © Bảo Nguyên,
phát hành theo giấy phép SIL Open Font License 1.1.
