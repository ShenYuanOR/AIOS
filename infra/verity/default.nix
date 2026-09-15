{ lib }:
{
  # Sign plugin/ISO closures on aios. Private keys stay in /var/lib/motherd/keys.
  pubkeyFile = "/var/lib/motherd/keys/plugins.pub";
  seckeyFile = "/var/lib/motherd/keys/plugins.sec";
}
