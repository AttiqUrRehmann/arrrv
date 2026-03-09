class Ruv < Formula
  desc "A fast R package manager, written in Rust (like uv for Python)"
  homepage "https://github.com/A-Fisk/ruv"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/A-Fisk/ruv/releases/download/v#{version}/ruv-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    end
    on_intel do
      url "https://github.com/A-Fisk/ruv/releases/download/v#{version}/ruv-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    url "https://github.com/A-Fisk/ruv/releases/download/v#{version}/ruv-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_LINUX_SHA256"
  end

  version "0.1.0-alpha.2"

  def install
    bin.install "ruv-#{version}/bin/ruv"
  end

  def post_install
    puts ""
    puts "ruv #{version} installed successfully!"
    puts ""
    puts "Quick start:"
    puts "  ruv --help"
    puts "  ruv install ggplot2"
    puts ""
  end

  test do
    system bin/"ruv", "--help"
  end
end
