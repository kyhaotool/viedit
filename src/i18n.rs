//! Toàn bộ chuỗi hiển thị của VieEdit.
//!
//! Mọi ngôn ngữ trong giao diện được gom về một chỗ. Mặc định là tiếng Việt,
//! có thể chuyển sang tiếng Anh trong menu "Trợ giúp → Ngôn ngữ".

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Vi,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Vi => "vi",
            Lang::En => "en",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Tr {
    pub app_name: &'static str,
    pub tagline: &'static str,
    pub untitled: &'static str,

    // Menu
    pub menu_file: &'static str,
    pub menu_edit: &'static str,
    pub menu_view: &'static str,
    pub menu_help: &'static str,
    pub menu_new: &'static str,
    pub menu_open: &'static str,
    pub menu_save: &'static str,
    pub menu_save_as: &'static str,
    pub menu_export: &'static str,
    pub menu_exit: &'static str,
    pub menu_undo: &'static str,
    pub menu_redo: &'static str,
    pub menu_language: &'static str,
    pub menu_about: &'static str,
    pub menu_shortcuts: &'static str,

    // Panels
    pub panel_library: &'static str,
    pub panel_preview: &'static str,
    pub panel_props: &'static str,
    pub panel_timeline: &'static str,

    // Library tabs
    pub tab_media: &'static str,
    pub tab_titles: &'static str,
    pub tab_transitions: &'static str,
    pub tab_gradients: &'static str,
    pub tab_social: &'static str,

    // Buttons
    pub btn_import: &'static str,
    pub btn_add_timeline: &'static str,
    pub btn_delete: &'static str,
    pub btn_split: &'static str,
    pub btn_duplicate: &'static str,
    pub btn_play: &'static str,
    pub btn_pause: &'static str,
    pub btn_start: &'static str,
    pub btn_end: &'static str,
    pub btn_export: &'static str,
    pub btn_cancel: &'static str,
    pub btn_close: &'static str,
    pub btn_choose: &'static str,
    pub btn_reload: &'static str,
    pub btn_folder: &'static str,
    pub btn_apply: &'static str,

    // Labels
    pub lbl_search: &'static str,
    pub lbl_time: &'static str,
    pub lbl_duration: &'static str,
    pub lbl_resolution: &'static str,
    pub lbl_fps: &'static str,
    pub lbl_volume: &'static str,
    pub lbl_opacity: &'static str,
    pub lbl_speed: &'static str,
    pub lbl_name: &'static str,
    pub lbl_source: &'static str,
    pub lbl_start: &'static str,
    pub lbl_end: &'static str,
    pub lbl_clip: &'static str,
    pub lbl_none: &'static str,
    pub lbl_zoom: &'static str,
    pub lbl_library_path: &'static str,
    pub lbl_quality: &'static str,
    pub lbl_format: &'static str,
    pub lbl_output: &'static str,

    // Tracks
    pub track_video: &'static str,
    pub track_overlay: &'static str,
    pub track_audio: &'static str,

    // Messages
    pub msg_empty_media: &'static str,
    pub msg_empty_library: &'static str,
    pub msg_drop_hint: &'static str,
    pub msg_select_clip: &'static str,
    pub msg_no_ffmpeg: &'static str,
    pub msg_ffmpeg_hint: &'static str,
    pub msg_importing: &'static str,
    pub msg_exporting: &'static str,
    pub msg_export_done: &'static str,
    pub msg_export_failed: &'static str,
    pub msg_saved: &'static str,
    pub msg_loaded: &'static str,
    pub msg_save_failed: &'static str,
    pub msg_nothing_to_export: &'static str,
    pub msg_unsupported: &'static str,
    pub msg_drag_tip: &'static str,

    // Dialogs
    pub dlg_about: &'static str,
    pub about_body: &'static str,
    pub shortcuts_body: &'static str,
    pub dlg_export: &'static str,

    // Misc
    pub yes: &'static str,
    pub no: &'static str,
    pub ok: &'static str,
    pub free: &'static str,
    pub pro: &'static str,
    pub seconds: &'static str,
}

pub fn tr(lang: Lang) -> Tr {
    match lang {
        Lang::Vi => VI,
        Lang::En => EN,
    }
}

pub const VI: Tr = Tr {
    app_name: "VieEdit",
    tagline: "Trình biên tập video tiếng Việt",
    untitled: "Dự án chưa đặt tên",

    menu_file: "Tệp",
    menu_edit: "Sửa",
    menu_view: "Hiển thị",
    menu_help: "Trợ giúp",
    menu_new: "Dự án mới",
    menu_open: "Mở dự án…",
    menu_save: "Lưu dự án",
    menu_save_as: "Lưu thành…",
    menu_export: "Xuất video…",
    menu_exit: "Thoát",
    menu_undo: "Hoàn tác",
    menu_redo: "Làm lại",
    menu_language: "Ngôn ngữ",
    menu_about: "Giới thiệu VieEdit",
    menu_shortcuts: "Phím tắt",

    panel_library: "Thư viện",
    panel_preview: "Xem trước",
    panel_props: "Thuộc tính",
    panel_timeline: "Dòng thời gian",

    tab_media: "Phương tiện",
    tab_titles: "Tiêu đề động",
    tab_transitions: "Chuyển cảnh",
    tab_gradients: "Nền chuyển sắc",
    tab_social: "Mạng xã hội",

    btn_import: "Nhập tệp",
    btn_add_timeline: "Thêm vào dòng thời gian",
    btn_delete: "Xóa",
    btn_split: "Cắt đôi",
    btn_duplicate: "Nhân bản",
    btn_play: "Phát",
    btn_pause: "Tạm dừng",
    btn_start: "Về đầu",
    btn_end: "Về cuối",
    btn_export: "Xuất video",
    btn_cancel: "Hủy",
    btn_close: "Đóng",
    btn_choose: "Chọn…",
    btn_reload: "Tải lại",
    btn_folder: "Thư mục",
    btn_apply: "Áp dụng",

    lbl_search: "Tìm kiếm",
    lbl_time: "Thời gian",
    lbl_duration: "Độ dài",
    lbl_resolution: "Độ phân giải",
    lbl_fps: "Khung hình/giây",
    lbl_volume: "Âm lượng",
    lbl_opacity: "Độ mờ",
    lbl_speed: "Tốc độ",
    lbl_name: "Tên",
    lbl_source: "Nguồn",
    lbl_start: "Bắt đầu",
    lbl_end: "Kết thúc",
    lbl_clip: "Đoạn phim",
    lbl_none: "Chưa chọn",
    lbl_zoom: "Thu phóng",
    lbl_library_path: "Thư mục thư viện",
    lbl_quality: "Chất lượng",
    lbl_format: "Định dạng",
    lbl_output: "Tệp xuất",

    track_video: "Hình ảnh",
    track_overlay: "Lớp phủ",
    track_audio: "Âm thanh",

    msg_empty_media: "Chưa có tệp nào. Nhấn “Nhập tệp” để bắt đầu.",
    msg_empty_library: "Không tìm thấy tài nguyên trong thư mục này.",
    msg_drop_hint: "Chọn một tài nguyên ở thư viện rồi thêm vào dòng thời gian, hoặc kéo trực tiếp.",
    msg_select_clip: "Chọn một đoạn trên dòng thời gian để chỉnh sửa.",
    msg_no_ffmpeg: "Chưa tìm thấy FFmpeg trên máy.",
    msg_ffmpeg_hint: "Cài FFmpeg để xem trước và xuất video: winget install Gyan.FFmpeg (Windows) hoặc sudo dnf install ffmpeg (Fedora).",
    msg_importing: "Đang đọc tệp…",
    msg_exporting: "Đang xuất video…",
    msg_export_done: "Xuất video thành công.",
    msg_export_failed: "Xuất video thất bại.",
    msg_saved: "Đã lưu dự án.",
    msg_loaded: "Đã mở dự án.",
    msg_save_failed: "Không lưu được dự án.",
    msg_nothing_to_export: "Dòng thời gian đang trống.",
    msg_unsupported: "Định dạng tệp không được hỗ trợ.",
    msg_drag_tip: "Kéo thân đoạn để di chuyển · kéo hai mép để cắt · Space để phát/dừng",

    dlg_about: "Giới thiệu",
    about_body: "VieEdit là trình biên tập video mã nguồn mở, viết hoàn toàn bằng Rust.\n\
                 Chạy trên Windows và Fedora Linux.\n\n\
                 Giao diện tiếng Việt, thư viện tiêu đề động đóng gói sẵn,\n\
                 xuất video qua FFmpeg.\n\n\
                 Giấy phép MIT.",
    shortcuts_body: "Space — phát / tạm dừng\n\
                     S — cắt đôi đoạn tại con trỏ\n\
                     Delete — xóa đoạn đang chọn\n\
                     Ctrl+S — lưu dự án\n\
                     Ctrl+E — xuất video\n\
                     Ctrl+Z — hoàn tác\n\
                     ← → — dịch con trỏ 1/30 giây\n\
                     Home / End — về đầu / cuối",
    dlg_export: "Xuất video",

    yes: "Có",
    no: "Không",
    ok: "Đồng ý",
    free: "Miễn phí",
    pro: "Chuyên nghiệp",
    seconds: "giây",
};

pub const EN: Tr = Tr {
    app_name: "VieEdit",
    tagline: "Vietnamese-first video editor",
    untitled: "Untitled project",

    menu_file: "File",
    menu_edit: "Edit",
    menu_view: "View",
    menu_help: "Help",
    menu_new: "New project",
    menu_open: "Open project…",
    menu_save: "Save project",
    menu_save_as: "Save as…",
    menu_export: "Export video…",
    menu_exit: "Quit",
    menu_undo: "Undo",
    menu_redo: "Redo",
    menu_language: "Language",
    menu_about: "About VieEdit",
    menu_shortcuts: "Keyboard shortcuts",

    panel_library: "Library",
    panel_preview: "Preview",
    panel_props: "Properties",
    panel_timeline: "Timeline",

    tab_media: "Media",
    tab_titles: "Motion titles",
    tab_transitions: "Transitions",
    tab_gradients: "Gradients",
    tab_social: "Social",

    btn_import: "Import",
    btn_add_timeline: "Add to timeline",
    btn_delete: "Delete",
    btn_split: "Split",
    btn_duplicate: "Duplicate",
    btn_play: "Play",
    btn_pause: "Pause",
    btn_start: "Go to start",
    btn_end: "Go to end",
    btn_export: "Export video",
    btn_cancel: "Cancel",
    btn_close: "Close",
    btn_choose: "Choose…",
    btn_reload: "Reload",
    btn_folder: "Folder",
    btn_apply: "Apply",

    lbl_search: "Search",
    lbl_time: "Time",
    lbl_duration: "Duration",
    lbl_resolution: "Resolution",
    lbl_fps: "FPS",
    lbl_volume: "Volume",
    lbl_opacity: "Opacity",
    lbl_speed: "Speed",
    lbl_name: "Name",
    lbl_source: "Source",
    lbl_start: "Start",
    lbl_end: "End",
    lbl_clip: "Clip",
    lbl_none: "None",
    lbl_zoom: "Zoom",
    lbl_library_path: "Library folder",
    lbl_quality: "Quality",
    lbl_format: "Format",
    lbl_output: "Output file",

    track_video: "Video",
    track_overlay: "Overlay",
    track_audio: "Audio",

    msg_empty_media: "No media yet. Press “Import” to start.",
    msg_empty_library: "No assets found in this folder.",
    msg_drop_hint: "Pick an asset in the library and add it to the timeline.",
    msg_select_clip: "Select a clip on the timeline to edit it.",
    msg_no_ffmpeg: "FFmpeg was not found.",
    msg_ffmpeg_hint: "Install FFmpeg to preview and export: winget install Gyan.FFmpeg (Windows) or sudo dnf install ffmpeg (Fedora).",
    msg_importing: "Reading file…",
    msg_exporting: "Exporting…",
    msg_export_done: "Export finished.",
    msg_export_failed: "Export failed.",
    msg_saved: "Project saved.",
    msg_loaded: "Project opened.",
    msg_save_failed: "Could not save the project.",
    msg_nothing_to_export: "The timeline is empty.",
    msg_unsupported: "Unsupported file format.",
    msg_drag_tip: "Drag the body to move · drag edges to trim · Space to play/pause",

    dlg_about: "About",
    about_body: "VieEdit is an open-source video editor written entirely in Rust.\n\
                 Runs on Windows and Fedora Linux.\n\n\
                 Vietnamese-first interface, bundled motion-title library,\n\
                 FFmpeg powered export.\n\n\
                 MIT licensed.",
    shortcuts_body: "Space — play / pause\n\
                     S — split clip at playhead\n\
                     Delete — delete selected clip\n\
                     Ctrl+S — save project\n\
                     Ctrl+E — export video\n\
                     Ctrl+Z — undo\n\
                     ← → — nudge playhead by 1/30 s\n\
                     Home / End — go to start / end",
    dlg_export: "Export video",

    yes: "Yes",
    no: "No",
    ok: "OK",
    free: "Free",
    pro: "Pro",
    seconds: "seconds",
};
