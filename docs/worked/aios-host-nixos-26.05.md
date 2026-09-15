# 工地机 NixOS 26.05

工地装机当轮归档。SQM1670 / Ryzen 7 8845HS 整盘清 Windows，官方 Minimal ISO（Rufus DD）SSH 装成构建机。

- 主机 `aios`，NixOS 26.05.9729，内核 6.18.51
- `192.168.110.99` 有线 `enp3s0`，sshd + `/dev/kvm`
- 盘：EFI 1G + swap 16G + root 914.5G ext4
- 这是工地，不是产品 ISO
