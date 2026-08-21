import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Locale = "en" | "vi";
type TranslationValue = string | Record<string, string>;

const resources: Record<Locale, Record<string, TranslationValue>> = {
  en: {
    "common.settings": "Settings",
    "common.language": "Language",
    "common.english": "English",
    "common.vietnamese": "Vietnamese",
    "common.save": "Save",
    "common.remove": "Remove",
    "common.testConnection": "Test connection",
    "common.loading": "Loading…",
    "common.error": "Something went wrong",
    "common.cancel": "Cancel",
    "common.close": "Close",
    "common.refresh": "Refresh",
    "common.available": "Available",
    "common.unavailable": "Unavailable",
    "nav.home": "Home",
    "nav.editor": "Editor",
    "nav.newProject": "New Project",
    "nav.openProject": "Open Project",
    "nav.save": "Save",
    "nav.saveAs": "Save As",
    "nav.saving": "Saving…",
    "nav.saved": "All changes saved",
    "nav.saveFailed": "Save failed",
    "nav.unsaved": "Unsaved changes",
    "nav.toggleTheme": "Toggle theme",
    "nav.notifications": "Notifications",
    "home.welcome": "Welcome to CutCut",
    "home.emptyDescription":
      "Please create a new project or open an existing one from the sidebar.",
    "home.title": "CutCut Media Toolchain",
    "home.description":
      "Local file dialog, FFprobe metadata, local video preview and FFmpeg export jobs.",
    "home.offline": "Offline / Signed Out",
    "home.sessionExpired": "Session expired — local mode",
    "home.cloudUnavailable": "Cloud unavailable — local mode",
    "home.signIn": "Sign In",
    "home.signOut": "Sign Out",
    "home.undo": "Undo",
    "home.redo": "Redo",
    "settings.languageDescription":
      "Choose the interface language. Project and transcript data are not translated.",
    "settings.cacheTitle": "Cache lifecycle",
    "settings.cacheDescription":
      "Only app-generated cache in .cutcut/artifacts is removed. Source media, Project JSON and user-owned assets are not touched.",
    "settings.openProjectForCache": "Open a project to inspect and clean cache.",
    "settings.reclaimable": "Reclaimable:",
    "settings.clearCache": "Clear regenerable cache",
    "settings.clearingCache": "Cleaning cache…",
    "settings.cacheCleared": "Released {bytes} of regenerable cache.",
    "settings.runtimeTitle": "Runtime diagnostics",
    "settings.runtimeDescription":
      "Capabilities are probed from the bundled Whisper runtime; GPU names do not advertise an unsupported backend.",
    "settings.refreshRuntime": "Refresh",
    "settings.checkingRuntime": "Checking…",
    "settings.cpu": "CPU",
    "settings.ram": "RAM",
    "settings.whisperRuntime": "Whisper runtime",
    "settings.backend": "Backend",
    "settings.modelTiers": "Model tiers",
    "settings.gpu": "GPU",
    "settings.noGpu": "Not detected",
    "settings.readingRuntime": "Reading runtime capabilities…",
    "settings.fallback": "Fallback:",
    "settings.speechTitle": "Speech preset",
    "settings.speechDescription":
      "Choose a speed/quality target; models are used only when installed and compatible with the current runtime.",
    "settings.advancedOverride": "Advanced model override (optional)",
    "settings.savePreset": "Save preset",
    "settings.savingPreset": "Applying…",
    "settings.recommendation": "Recommendation:",
    "settings.modelNotReady":
      "Model is not ready — open AI Models to download a model or choose a fallback.",
    "settings.aiTitle": "AI provider",
    "settings.hostedMode": "Hosted trial/credits",
    "settings.byokMode": "Gemini BYOK",
    "settings.keyLabel": "Gemini API key",
    "settings.keyPlaceholder": "Paste the key once to store it in the OS",
    "settings.keyConfigured": "configured",
    "settings.keyNotConfigured": "not configured",
    "settings.saveKey": "Store key securely",
    "settings.removeKey": "Remove key",
    "settings.keySaved": "Key stored in the OS credential store.",
    "settings.connectionSuccess": "Gemini connection succeeded.",
    "settings.byokDescription":
      "BYOK sends transcript text to Google with your key; video and audio remain local. The full key is not stored in the project, localStorage, or logs.",
    "errors.not_configured": "Configure a Gemini key first.",
    "errors.invalid_key": "The Gemini key was rejected.",
    "errors.provider_unavailable": "The AI provider is temporarily unavailable.",
    "errors.rate_limited": "The provider rate limit was reached.",
    "errors.invalid_provider_output": "The provider returned an invalid edit plan.",
    "errors.generic": "The operation could not be completed.",
    "editor.reviewSuggestions": "Review Suggestions",
    "editor.analyzing": "Analyzing…",
    "editor.generateAnalysis": "Generate Analysis",
    "editor.aiAnalyzing": "AI analyzing…",
    "editor.analyzeByok": "Analyze with BYOK",
    "editor.analyzeHosted": "Analyze with hosted AI",
    "editor.noAnalysis":
      "No local analysis is available for this media. Run analysis to create suggestions.",
    "editor.runAnalysisHint": 'Press "Generate Analysis" to create suggestions.',
    "editor.keepAll": "Keep All (No Cuts)",
    "editor.applyAll": "Apply All Cuts",
    "editor.reviewRequired": "Review required — segment timing",
    "editor.remove": "Remove",
    "editor.keep": "Keep",
    "editor.skip": "Skip",
    "editor.cut": "Cut",
    "editor.highlight": "Highlight",
    "editor.action": "Action",
    "editor.noTranscript": "No transcript available.",
    "editor.runTranscription": "Run AI Transcription to generate segments.",
    "editor.resumeAutoScroll": "Resume Auto-scroll",
    "editor.silence": "Silence",
    "editor.backgroundNoise": "Background Noise",
    "editor.uncertainSpeech": "Uncertain Speech",
    "editor.filler": "Filler",
    "editor.previewRange": "Preview {start} to {end}",
    "editor.falseStart": "False Start",
    "editor.repeatedTake": "Repeated Take",
    "editor.redundant": "Redundant",
    "editor.confident": "% confident",
    "editor.transcript": "Transcript",
    "editor.videoPreview": "Video Preview",
    "editor.captions": "Captions",
    "editor.toggleCaptions": "Toggle captions",
    "editor.previewCuts": "Preview Cuts",
    "editor.sourcePreview": "Source-time preview — enabled cuts will be skipped.",
    "editor.exportTitle": "Export Prototype",
    "editor.previewStart": "Preview start (ms)",
    "editor.usePlayhead": "Use playhead",
    "editor.previewRangeHelp":
      "Accurate Preview clamps the range to 3–5 seconds in the source timeline.",
    "editor.exportMp4": "Export to MP4",
    "editor.accuratePreview": "Accurate Preview (3–5s)",
    "editor.renderingPreview": "Rendering preview ({percent}%)",
    "editor.cancelPreview": "Cancel preview",
    "editor.exporting": "Exporting…",
    "editor.cancel": "Cancel",
    "editor.renderedPreview": "Rendered preview",
    "editor.cachedRender": "Cached by render signature.",
    "editor.cutPreviewToggle": "Toggle cut-preview mode",
    "editor.removeAllCuts": "Remove All Cuts",
    "editor.filterAll": "All",
    "editor.filterLocal": "Local",
    "editor.filterAi": "AI Only",
    "editor.filterUser": "User",
    "editor.noFilterActions": "No actions match the selected filter.",
    "editor.generateHint": "Click 'Generate Analysis' to find removable segments.",
    "errors.cloudPlan": "Your current plan does not include Cloud AI features. Please upgrade.",
    "editor.startPlayback": "Start Playback",
    "editor.pausePlayback": "Pause Playback",
    "editor.seek10": "+10s Seek",
    "editor.silenceConfig": "Configure silence",
    "editor.noProject": "Video Preview Area",
    "auth.createAccount": "Create an Account",
    "auth.signIn": "Sign In",
    "auth.cloudDescription": "Sign in to access your Cloud AI features.",
    "auth.cloudSignupDescription": "Sign up to access Cloud AI features.",
    "auth.email": "Email",
    "auth.password": "Password",
    "auth.wait": "Please wait…",
    "auth.signUp": "Sign Up",
    "auth.alreadyAccount": "Already have an account? Sign In",
    "auth.needAccount": "Need an account? Sign Up",
    "auth.failed": "Authentication failed",
    "settings.telemetryTitle": "Privacy-safe telemetry",
    "settings.telemetryDescription":
      "Only allowlisted events and aggregate metrics are queued. Video, audio, transcript, project paths, API keys and raw error content are never sent.",
    "settings.telemetryOptIn": "Allow redacted telemetry and crash diagnostics",
    "settings.telemetryDisabled": "Disabling this option clears the local queue.",
    "settings.updaterTitle": "Updater",
    "settings.checkUpdates": "Check for updates",
    "settings.checkingUpdates": "Checking…",
    "settings.upToDate": "You are up to date.",
    "settings.updateAvailable": "Update available: {version}",
    "settings.updaterUnconfigured":
      "Updater is disabled until the release signing key and trusted endpoint are configured.",
    "settings.updateError": "Update check failed. The current app remains usable.",
    "settings.aiModels": "AI Models",
    "settings.speechModels": "Speech model manager",
    "settings.modelCompatibility":
      "A model is selectable only after checksum and CPU runtime compatibility pass.",
    "settings.diskUsage": "Using {size} in the app data directory.",
    "settings.retryDownload": "Retry download",
    "settings.download": "Download",
    "settings.useModel": "Use",
    "settings.downloading": "Downloading…",
    "settings.incompatible": "Incompatible:",
    "settings.checksumError": "Checksum/manifest error:",
    "settings.downloadFailed": "Model download failed:",
    "settings.silenceTitle": "Configure silence detection",
    "settings.silenceDescription":
      "Adjust silence detection sensitivity. These settings determine which ranges are suggested for removal.",
    "settings.conservative": "Conservative",
    "settings.conservativeDescription": "Safest: only cuts very clear, long silences.",
    "settings.balanced": "Balanced (recommended)",
    "settings.balancedDescription":
      "Suitable for most vlog/podcast videos and standard breath pauses.",
    "settings.aggressive": "Aggressive",
    "settings.aggressiveDescription": "Cuts short pauses for fast videos, but may clip breaths.",
    "settings.custom": "Custom (advanced)",
    "settings.customDescription": "Set the volume threshold and minimum duration yourself.",
    "settings.volumeThreshold": "Volume threshold",
    "settings.minimumDuration": "Minimum duration",
    "settings.noActiveVideo": "No active video to test. Import a video first.",
    "settings.silenceFailed": "Silence detection failed:",
    "settings.runningTest": "Running…",
    "settings.runPreview": "Run preview",
    "settings.expectedCuts": "Expected cuts: {count} segments ({duration}s)",
    "settings.saveConfiguration": "Save configuration",
    "media.missingTitle": "Media file missing",
    "media.missingDescription": "The source media cannot be found at its original location:",
    "media.editsSafe":
      "Your project edits are safe, but relink the media to continue editing or exporting.",
    "media.relinkWarning": "Replacement media is not fully compatible",
    "media.relinkWarningDetail":
      "Transcript and Edit Plan keep the old source timestamps. Continue only if the timeline change is acceptable.",
    "media.relinkWithWarning": "Relink with warning",
    "media.chooseAnother": "Choose another file",
    "media.relink": "Relink file",
    "media.relinking": "Relinking…",
    "media.importerTitle": "Media importer",
    "media.readingMetadata": "Reading metadata…",
    "media.selectVideo": "Select video file",
    "media.metadataParsed": "Metadata parsed",
    "common.reload": "Reload",
    "errors.appCrashTitle": "CutCut encountered an unexpected error",
    "errors.appCrashDescription": "Your current session is safe. Try reloading the interface.",
  },
  vi: {
    "common.settings": "Cài đặt",
    "common.language": "Ngôn ngữ",
    "common.english": "Tiếng Anh",
    "common.vietnamese": "Tiếng Việt",
    "common.save": "Lưu",
    "common.remove": "Xóa",
    "common.testConnection": "Kiểm tra kết nối",
    "common.loading": "Đang tải…",
    "common.error": "Đã xảy ra lỗi",
    "common.cancel": "Hủy",
    "common.close": "Đóng",
    "common.refresh": "Làm mới",
    "common.available": "Khả dụng",
    "common.unavailable": "Không khả dụng",
    "nav.home": "Trang chủ",
    "nav.editor": "Biên tập",
    "nav.newProject": "Project mới",
    "nav.openProject": "Mở project",
    "nav.save": "Lưu",
    "nav.saveAs": "Lưu thành",
    "nav.saving": "Đang lưu…",
    "nav.saved": "Đã lưu mọi thay đổi",
    "nav.saveFailed": "Lưu thất bại",
    "nav.unsaved": "Có thay đổi chưa lưu",
    "nav.toggleTheme": "Đổi giao diện sáng/tối",
    "nav.notifications": "Thông báo",
    "home.welcome": "Chào mừng đến CutCut",
    "home.emptyDescription": "Tạo project mới hoặc mở project hiện có từ thanh bên.",
    "home.title": "CutCut Media Toolchain",
    "home.description":
      "Dialog file local, metadata FFprobe, preview video local và job export FFmpeg.",
    "home.offline": "Offline / Đã đăng xuất",
    "home.sessionExpired": "Session hết hạn — chế độ local",
    "home.cloudUnavailable": "Cloud không khả dụng — chế độ local",
    "home.signIn": "Đăng nhập",
    "home.signOut": "Đăng xuất",
    "home.undo": "Hoàn tác",
    "home.redo": "Làm lại",
    "settings.languageDescription":
      "Chọn ngôn ngữ giao diện. Dữ liệu project và transcript không bị dịch.",
    "settings.cacheTitle": "Vòng đời cache",
    "settings.cacheDescription":
      "Chỉ dọn cache do app tạo trong .cutcut/artifacts. Source media, Project JSON và tài sản của user không bị chạm tới.",
    "settings.openProjectForCache": "Mở một project để xem và dọn cache.",
    "settings.reclaimable": "Có thể giải phóng:",
    "settings.clearCache": "Xóa cache có thể tạo lại",
    "settings.clearingCache": "Đang dọn cache…",
    "settings.cacheCleared": "Đã giải phóng {bytes} cache có thể tạo lại.",
    "settings.runtimeTitle": "Chẩn đoán runtime",
    "settings.runtimeDescription":
      "Capability được probe từ runtime Whisper đã bundle; GPU name không tự quảng cáo backend không hỗ trợ.",
    "settings.refreshRuntime": "Làm mới",
    "settings.checkingRuntime": "Đang kiểm tra…",
    "settings.cpu": "CPU",
    "settings.ram": "RAM",
    "settings.whisperRuntime": "Whisper runtime",
    "settings.backend": "Backend",
    "settings.modelTiers": "Model tiers",
    "settings.gpu": "GPU",
    "settings.noGpu": "Không phát hiện",
    "settings.readingRuntime": "Đang đọc capability runtime…",
    "settings.fallback": "Fallback:",
    "settings.speechTitle": "Speech preset",
    "settings.speechDescription":
      "Chọn mục tiêu tốc độ/chất lượng; model chỉ được dùng khi đã cài và tương thích với runtime hiện tại.",
    "settings.advancedOverride": "Advanced model override (tùy chọn)",
    "settings.savePreset": "Lưu preset",
    "settings.savingPreset": "Đang áp dụng…",
    "settings.recommendation": "Đề xuất:",
    "settings.modelNotReady": "Model chưa sẵn sàng — mở AI Models để tải model hoặc chọn fallback.",
    "settings.aiTitle": "Nhà cung cấp AI",
    "settings.hostedMode": "Hosted trial/credits",
    "settings.byokMode": "Gemini BYOK",
    "settings.keyLabel": "Gemini API key",
    "settings.keyPlaceholder": "Dán key một lần để lưu vào OS",
    "settings.keyConfigured": "đã cấu hình",
    "settings.keyNotConfigured": "chưa cấu hình",
    "settings.saveKey": "Lưu key an toàn",
    "settings.removeKey": "Xóa key",
    "settings.keySaved": "Đã lưu key trong OS credential store.",
    "settings.connectionSuccess": "Kết nối Gemini thành công.",
    "settings.byokDescription":
      "BYOK gửi transcript tới Google bằng key của bạn; video và audio vẫn xử lý local. Full key không nằm trong project, localStorage hoặc log.",
    "errors.not_configured": "Hãy cấu hình Gemini key trước.",
    "errors.invalid_key": "Gemini đã từ chối key.",
    "errors.provider_unavailable": "Nhà cung cấp AI hiện không khả dụng.",
    "errors.rate_limited": "Nhà cung cấp đang giới hạn tốc độ.",
    "errors.invalid_provider_output": "Provider trả về edit plan không hợp lệ.",
    "errors.generic": "Không thể hoàn tất thao tác.",
    "editor.reviewSuggestions": "Review đề xuất",
    "editor.analyzing": "Đang phân tích…",
    "editor.generateAnalysis": "Tạo phân tích",
    "editor.aiAnalyzing": "AI đang phân tích…",
    "editor.analyzeByok": "Phân tích bằng BYOK",
    "editor.analyzeHosted": "Phân tích bằng hosted AI",
    "editor.noAnalysis": "Chưa có local analysis cho media này. Hãy chạy phân tích để tạo đề xuất.",
    "editor.runAnalysisHint": 'Nhấn "Tạo phân tích" để tạo đề xuất.',
    "editor.keepAll": "Giữ tất cả (Không cắt)",
    "editor.applyAll": "Áp dụng tất cả đoạn cắt",
    "editor.reviewRequired": "Cần review — timing segment",
    "editor.remove": "Xóa",
    "editor.keep": "Giữ",
    "editor.skip": "Bỏ qua",
    "editor.cut": "Cắt",
    "editor.highlight": "Đánh dấu",
    "editor.action": "Action",
    "editor.noTranscript": "Chưa có transcript.",
    "editor.runTranscription": "Chạy AI Transcription để tạo segment.",
    "editor.resumeAutoScroll": "Bật lại tự động cuộn",
    "editor.silence": "Khoảng lặng",
    "editor.backgroundNoise": "Tạp âm nền",
    "editor.uncertainSpeech": "Giọng nói không chắc chắn",
    "editor.filler": "Từ đệm",
    "editor.previewRange": "Preview {start} đến {end}",
    "editor.falseStart": "Nói hụt",
    "editor.repeatedTake": "Take lặp",
    "editor.redundant": "Trùng ý",
    "editor.confident": "% tin cậy",
    "editor.transcript": "Transcript",
    "editor.videoPreview": "Video Preview",
    "editor.captions": "Captions",
    "editor.toggleCaptions": "Bật/tắt caption",
    "editor.previewCuts": "Preview đoạn cắt",
    "editor.sourcePreview": "Preview theo thời gian source — đoạn cắt bật sẽ được bỏ qua.",
    "editor.exportTitle": "Export Prototype",
    "editor.previewStart": "Bắt đầu preview (ms)",
    "editor.usePlayhead": "Dùng playhead",
    "editor.previewRangeHelp":
      "Accurate Preview tự clamp range còn 3–5 giây trong source timeline.",
    "editor.exportMp4": "Export MP4",
    "editor.accuratePreview": "Accurate Preview (3–5s)",
    "editor.renderingPreview": "Đang render preview ({percent}%)",
    "editor.cancelPreview": "Hủy preview",
    "editor.exporting": "Đang export…",
    "editor.cancel": "Hủy",
    "editor.renderedPreview": "Preview đã render",
    "editor.cachedRender": "Đã cache theo render signature.",
    "editor.cutPreviewToggle": "Bật/tắt chế độ preview đoạn cắt",
    "editor.removeAllCuts": "Xóa tất cả đoạn cắt",
    "editor.filterAll": "Tất cả",
    "editor.filterLocal": "Local",
    "editor.filterAi": "Chỉ AI",
    "editor.filterUser": "User",
    "editor.noFilterActions": "Không có action khớp bộ lọc đã chọn.",
    "editor.generateHint": "Nhấn 'Tạo phân tích' để tìm các đoạn có thể cắt.",
    "errors.cloudPlan": "Gói hiện tại chưa có Cloud AI. Hãy nâng cấp để tiếp tục.",
    "editor.startPlayback": "Phát",
    "editor.pausePlayback": "Tạm dừng",
    "editor.seek10": "+10s",
    "editor.silenceConfig": "Cấu hình khoảng lặng",
    "editor.noProject": "Khu vực Video Preview",
    "auth.createAccount": "Tạo tài khoản",
    "auth.signIn": "Đăng nhập",
    "auth.cloudDescription": "Đăng nhập để dùng tính năng Cloud AI.",
    "auth.cloudSignupDescription": "Đăng ký để dùng tính năng Cloud AI.",
    "auth.email": "Email",
    "auth.password": "Mật khẩu",
    "auth.wait": "Đang xử lý…",
    "auth.signUp": "Đăng ký",
    "auth.alreadyAccount": "Đã có tài khoản? Đăng nhập",
    "auth.needAccount": "Chưa có tài khoản? Đăng ký",
    "auth.failed": "Xác thực thất bại",
    "settings.telemetryTitle": "Telemetry bảo vệ riêng tư",
    "settings.telemetryDescription":
      "Chỉ xếp hàng event allowlist và số liệu tổng hợp. Video, audio, transcript, đường dẫn project, API key và nội dung lỗi nguyên bản không được gửi.",
    "settings.telemetryOptIn": "Cho phép telemetry và crash diagnostics đã redaction",
    "settings.telemetryDisabled": "Tắt tùy chọn sẽ xóa queue local.",
    "settings.updaterTitle": "Updater",
    "settings.checkUpdates": "Kiểm tra cập nhật",
    "settings.checkingUpdates": "Đang kiểm tra…",
    "settings.upToDate": "App đã là phiên bản mới nhất.",
    "settings.updateAvailable": "Có bản cập nhật: {version}",
    "settings.updaterUnconfigured":
      "Updater bị tắt cho tới khi cấu hình signing key và trusted endpoint.",
    "settings.updateError": "Kiểm tra cập nhật thất bại. App hiện tại vẫn dùng được.",
    "settings.aiModels": "AI Models",
    "settings.speechModels": "Quản lý Speech Models",
    "settings.modelCompatibility":
      "Model chỉ được chọn sau khi checksum và compatibility với runtime CPU pass.",
    "settings.diskUsage": "Đang dùng {size} trong app data directory.",
    "settings.retryDownload": "Tải lại",
    "settings.download": "Tải về",
    "settings.useModel": "Dùng",
    "settings.downloading": "Đang tải…",
    "settings.incompatible": "Không tương thích:",
    "settings.checksumError": "Checksum/manifest lỗi:",
    "settings.downloadFailed": "Tải model thất bại:",
    "settings.silenceTitle": "Cấu hình nhận diện khoảng lặng",
    "settings.silenceDescription":
      "Điều chỉnh độ nhạy khi phát hiện khoảng lặng. Thiết lập này xác định đoạn nào được đề xuất cắt bỏ.",
    "settings.conservative": "Cẩn trọng",
    "settings.conservativeDescription": "An toàn nhất: chỉ cắt khoảng lặng rất rõ ràng và dài.",
    "settings.balanced": "Cân bằng (khuyên dùng)",
    "settings.balancedDescription":
      "Phù hợp với đa số video vlog/podcast và khoảng nghỉ tiêu chuẩn.",
    "settings.aggressive": "Tích cực",
    "settings.aggressiveDescription":
      "Cắt sát nhịp nghỉ cho video nhanh nhưng có thể cắt lẹm hơi thở.",
    "settings.custom": "Tùy chỉnh (nâng cao)",
    "settings.customDescription": "Tự thiết lập ngưỡng âm lượng và thời gian tối thiểu.",
    "settings.volumeThreshold": "Ngưỡng âm lượng",
    "settings.minimumDuration": "Thời gian tối thiểu",
    "settings.noActiveVideo": "Chưa có video để test. Hãy import video trước.",
    "settings.silenceFailed": "Nhận diện khoảng lặng thất bại:",
    "settings.runningTest": "Đang chạy…",
    "settings.runPreview": "Chạy thử (Preview)",
    "settings.expectedCuts": "Dự kiến cắt: {count} đoạn ({duration}s)",
    "settings.saveConfiguration": "Lưu cấu hình",
    "media.missingTitle": "Thiếu file media",
    "media.missingDescription": "Không tìm thấy source media tại vị trí ban đầu:",
    "media.editsSafe":
      "Các chỉnh sửa project vẫn an toàn, nhưng cần relink media để tiếp tục edit hoặc export.",
    "media.relinkWarning": "Media thay thế không hoàn toàn tương thích",
    "media.relinkWarningDetail":
      "Transcript và Edit Plan vẫn giữ timestamp source cũ. Chỉ tiếp tục nếu bạn chấp nhận timeline có thể thay đổi.",
    "media.relinkWithWarning": "Relink với cảnh báo",
    "media.chooseAnother": "Chọn file khác",
    "media.relink": "Relink file",
    "media.relinking": "Đang relink…",
    "media.importerTitle": "Import media",
    "media.readingMetadata": "Đang đọc metadata…",
    "media.selectVideo": "Chọn file video",
    "media.metadataParsed": "Metadata đã đọc",
    "common.reload": "Tải lại",
    "errors.appCrashTitle": "CutCut gặp lỗi ngoài dự kiến",
    "errors.appCrashDescription": "Phiên làm việc hiện tại vẫn an toàn. Hãy thử tải lại giao diện.",
  },
};

const localeStorageKey = "cutcut-locale";

function detectLocale(): Locale {
  if (typeof window === "undefined") return "vi";
  const stored = window.localStorage.getItem(localeStorageKey);
  if (stored === "en" || stored === "vi") return stored;
  return window.navigator.language.toLowerCase().startsWith("vi") ? "vi" : "en";
}

function lookup(locale: Locale, key: string): string {
  const value = resources[locale][key] ?? resources.vi[key];
  return typeof value === "string" ? value : key;
}

export function translateErrorCode(value: unknown, locale: Locale): string {
  const code = typeof value === "string" ? value : "generic";
  return lookup(locale, `errors.${code}`) === `errors.${code}`
    ? lookup(locale, "errors.generic")
    : lookup(locale, `errors.${code}`);
}

export function formatNumber(value: number, locale: Locale): string {
  return new Intl.NumberFormat(locale === "vi" ? "vi-VN" : "en-US").format(value);
}

export function formatDate(value: Date | number, locale: Locale): string {
  return new Intl.DateTimeFormat(locale === "vi" ? "vi-VN" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(value);
}

export function formatDuration(milliseconds: number, locale: Locale): string {
  const totalSeconds = Math.max(0, Math.round(milliseconds / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return locale === "vi"
    ? `${minutes} phút ${seconds.toString().padStart(2, "0")} giây`
    : `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}

export function formatBytes(bytes: number, locale: Locale): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const formatted = new Intl.NumberFormat(locale === "vi" ? "vi-VN" : "en-US", {
    maximumFractionDigits: unit === 0 ? 0 : 1,
  }).format(value);
  return `${formatted} ${units[unit]}`;
}

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: string, values?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);
  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
    if (typeof window !== "undefined") window.localStorage.setItem(localeStorageKey, nextLocale);
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale,
      t: (key, values) => {
        const message = lookup(locale, key);
        return values
          ? message.replace(/\{(\w+)\}/g, (_, name: string) => String(values[name] ?? `{${name}}`))
          : message;
      },
    }),
    [locale, setLocale],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n must be used inside I18nProvider");
  return context;
}
