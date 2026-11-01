class Mossambi < Formula
  desc "Zero-loss native file converter, archive suite, and Indian exam studio"
  homepage "https://github.com/mossambi/mossambi"
  url "https://github.com/mossambi/mossambi/archive/refs/tags/v2.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/mossambi/mossambi.git", branch: "main"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build
  depends_on "ffmpeg"

  on_linux do
    depends_on "alsa-lib"
    depends_on "mesa"
    depends_on "libxkbcommon"
    depends_on "wayland"
  end

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/mossambi-cli")
    system "cargo", "install", *std_cargo_args(path: "crates/mossambi-tui")
    system "cargo", "install", *std_cargo_args(path: "crates/mossambi-gui")
    system "cargo", "install", *std_cargo_args(path: "crates/mossambi-widget")

    # Generate and install shell completions
    generate_completions_from_executable(bin/"mossambi", "completions")
  end

  test do
    assert_match "Mossambi", shell_output("#{bin}/mossambi --version")
    assert_match "Zero-loss native", shell_output("#{bin}/mossambi --help")
  end
end
