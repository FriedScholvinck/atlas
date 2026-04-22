class Atlas < Formula
  desc "Native control plane for your Mac's software stack"
  homepage "https://friedscholvinck.github.io/atlas"
  head "https://github.com/FriedScholvinck/atlas.git", branch: "main"
  license "MIT"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "atlas", shell_output("#{bin}/atlas --help")
  end
end
