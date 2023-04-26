{
  buildVimPluginFrom2Nix,
  fetchurl,
}: {
  dressing-nvim = buildVimPluginFrom2Nix {
    pname = "dressing-nvim"; # Manifest entry: "stevearc/dressing.nvim"
    version = "2023-04-22";
    src = fetchurl {
      url = "https://github.com/stevearc/dressing.nvim/archive/f5d7fa1fa5ce6bcebc8f07922f39b1eda4d01e37.tar.gz";
      sha256 = "0fzavzxpyl26y7mliaa06rf51p5ps9v3i1hhlwml0p0kf1psdxdk";
    };
  };
}
