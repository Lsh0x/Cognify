/// Constants used throughout the Cognifs application
/// This module centralizes all constant values for better maintainability

/// Patterns that indicate a directory should not be reorganized
/// These include version control systems and project structures
pub const PROTECTED_PATTERNS: &[&str] = &[
    // Version control systems
    ".git",
    ".hg",           // Mercurial
    ".svn",          // Subversion
    ".bzr",          // Bazaar
    "CVS",           // CVS
    ".fossil",       // Fossil
    // Build artifacts and dependencies
    "node_modules",
    "target",        // Rust
    "dist",
    "build",
    ".gradle",       // Gradle (Java/Kotlin)
    ".mvn",          // Maven
    "venv",          // Python virtual environment
    ".venv",
    "env",
    ".env",
    "__pycache__",
    ".pytest_cache",
    ".tox",
    ".mypy_cache",
    // Application bundles and packages (macOS, iOS, Linux)
    ".app",          // macOS application bundle (e.g., "MyApp.app")
    ".framework",    // macOS framework bundle (e.g., "MyFramework.framework")
    ".plugin",       // macOS plugin bundle (e.g., "MyPlugin.plugin")
    ".bundle",       // Generic bundle (macOS/Linux, e.g., "MyBundle.bundle")
    ".kext",         // macOS kernel extension (e.g., "MyKext.kext")
    ".xcarchive",    // Xcode archive (e.g., "MyApp.xcarchive")
    ".dSYM",         // Debug symbols bundle (e.g., "MyApp.dSYM")
    ".xcodeproj",    // Xcode project bundle
    ".xcworkspace",  // Xcode workspace bundle
    // Package/installer files (should not be unpacked/reorganized)
    ".pkg",          // macOS installer package
    ".deb",          // Debian package
    ".rpm",          // RPM package
    // Project configuration indicators (if present, protect the directory)
    "package.json",  // Node.js
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.toml",    // Rust
    "Cargo.lock",
    "go.mod",        // Go
    "go.sum",
    "requirements.txt", // Python
    "setup.py",
    "pyproject.toml",
    "pom.xml",       // Maven
    "build.gradle",  // Gradle
    "composer.json", // PHP
    "Gemfile",       // Ruby
    "docker-compose.yml",
    "Dockerfile",
    ".gitignore",
    ".gitattributes",
];

/// Bundle extensions that should be protected (can appear as suffixes)
pub const BUNDLE_EXTENSIONS: &[&str] = &[
    ".app",
    ".framework",
    ".plugin",
    ".bundle",
    ".kext",
    ".xcarchive",
    ".dSYM",
    ".xcodeproj",
    ".xcworkspace",
];

/// Protected directory patterns that should be checked inside directories
pub const PROTECTED_DIR_PATTERNS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".bzr",
    ".fossil",
    ".app",
    ".framework",
    ".plugin",
    ".bundle",
    ".kext",
    ".xcarchive",
    ".dSYM",
    ".xcodeproj",
    ".xcworkspace",
    "CVS",
];

/// Top-level category tags (file type or broad category) - Level 1
pub const TOP_LEVEL_CATEGORIES: &[&str] = &[
    "document",
    "image",
    "video",
    "audio",
    "archive",
    "spreadsheet",
    "programming",
    "task",
    "calendar",
    "financial",
    "reporting",
    "configuration",
    "testing",
    "integration",
    "enhancement",
    "issue",
    "notes",
    "draft",
    "meeting",
    "project",
    "work",
    "personal",
];

/// Mid-level subcategory tags (language, domain, type) - Level 2
pub const MID_LEVEL_CATEGORIES: &[&str] = &[
    // Languages/Technologies
    "rust",
    "python",
    "javascript",
    "java",
    "go",
    "cpp",
    "typescript",
    // Financial subcategories
    "invoice",
    "receipt",
    "statement",
    "bill",
    "payment",
    "tax",
    // Document types
    "report",
    "minutes",
    "agenda",
    "proposal",
    "contract",
    // Work categories
    "meeting",
    "notes",
    "tutorial",
    "guide",
    "documentation",
    "reference",
    // Project phases
    "test",
    "spec",
    "design",
    "plan",
    "draft",
    "final",
];

/// Specific/concrete tags (file purpose, content type) - Level 3+
pub const SPECIFIC_TAGS: &[&str] = &[
    "invoice",
    "receipt",
    "statement",
    "bill",
    "payment",
    "meeting",
    "notes",
    "minutes",
    "agenda",
    "tutorial",
    "guide",
    "howto",
    "readme",
    "changelog",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "2023",
    "2024",
    "2025",
];

/// File extensions for document types
pub const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf",
    "doc",
    "docx",
];

/// File extensions for image types
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg",
    "jpeg",
    "png",
    "gif",
    "webp",
    "heic",
];

/// File extensions for video types
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4",
    "avi",
    "mov",
    "mkv",
];

/// File extensions for audio types
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3",
    "wav",
    "flac",
    "m4a",
];

/// File extensions for archive types
pub const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip",
    "tar",
    "gz",
    "rar",
    "7z",
];

/// File extensions for spreadsheet types
pub const SPREADSHEET_EXTENSIONS: &[&str] = &[
    "xls",
    "xlsx",
    "csv",
];

/// Common directory names that should be ignored when extracting path context
pub const COMMON_DIRECTORY_NAMES: &[&str] = &[
    "documents",
    "downloads",
    "desktop",
    "pictures",
    "music",
    "videos",
    "home",
    "user",
    "users",
    "tmp",
    "temp",
    "cache",
    "data",
    "files",
    "folder",
    "folders",
    "file",
    "dir",
    "directory",
    "src",
    "lib",
    "code",
    "projects",
];

/// LLM keyword mappings for tag generation
pub const LLM_KEYWORD_MAPPINGS: &[(&str, &str)] = &[
    ("todo", "task"),
    ("meeting", "calendar"),
    ("code", "programming"),
    ("bug", "issue"),
    ("feature", "enhancement"),
    ("api", "integration"),
    ("invoice", "financial"),
    ("receipt", "financial"),
    ("report", "reporting"),
    ("notes", "notes"),
    ("draft", "draft"),
];

