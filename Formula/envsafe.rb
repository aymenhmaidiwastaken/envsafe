class Envsafe < Formula
  desc "Your secrets, encrypted, everywhere. Universal .env & secrets manager"
  homepage "https://github.com/aymenhmaidiwastaken/envsafe"
  version "0.2.0"
  license "MIT"

  # Platform-specific binaries
  on_macos do
    on_arm do
      url "https://github.com/aymenhmaidiwastaken/envsafe/releases/download/v#{version}/envsafe-aarch64-apple-darwin"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/aymenhmaidiwastaken/envsafe/releases/download/v#{version}/envsafe-x86_64-apple-darwin"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/aymenhmaidiwastaken/envsafe/releases/download/v#{version}/envsafe-aarch64-unknown-linux-gnu"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/aymenhmaidiwastaken/envsafe/releases/download/v#{version}/envsafe-x86_64-unknown-linux-gnu"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "envsafe-*" => "envsafe"
  end

  test do
    system "#{bin}/envsafe", "--version"
  end
end
