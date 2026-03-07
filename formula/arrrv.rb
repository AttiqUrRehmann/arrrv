class Arrrv < Formula
  desc "A fast R package manager, written in Rust (like uv for Python)"
  homepage "https://github.com/A-Fisk/arrrv"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/A-Fisk/arrrv/releases/download/v#{version}/arrrv-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    end
    on_intel do
      url "https://github.com/A-Fisk/arrrv/releases/download/v#{version}/arrrv-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    url "https://github.com/A-Fisk/arrrv/releases/download/v#{version}/arrrv-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_LINUX_SHA256"
  end

  version "0.1.0"

  def install
    bin.install "arrrv-#{version}/bin/arrrv"
  end

  def post_install
    puts ""
    puts "arrrv #{version} installed successfully!"
    puts ""
    puts "Quick start:"
    puts "  arrrv --help"
    puts "  arrrv install ggplot2"
    puts ""
  end

  test do
    system bin/"arrrv", "--help"
  end
end
