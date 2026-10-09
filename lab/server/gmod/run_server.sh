#!/bin/sh
# Garry's Mod dedicated server for the Signet door: LAN only, on this machine only, VAC off (our own server).
#   ~/signet-server/gmod/run_server.sh
# then in Garry's Mod, console:  net_usesocketsforloopback 1 ; connect 127.0.0.1:27015
#   (or launch option +net_usesocketsforloopback 1: the client has the same 127.0.0.1 filter as the server)
# net_usesocketsforloopback 1: without it Source drops every packet coming from 127.0.0.1 (it expects a listen
# server's in-process loopback), so a game on this same PC could not even see the server.
# Needs: the dedicated server in ~/gmod-server/ds (SteamCMD app 4020) and the addon linked:
#   ln -sfn ~/signet-server/gmod/signet ~/gmod-server/ds/garrysmod/addons/signet
# and a Signet Link with the GMod door:  signet link --gmod [--server HOST:7800]
DS="${GMOD_DS:-$HOME/gmod-server/ds}"
cd "$DS" || exit 1
exec ./srcds_run -game garrysmod -console -condebug -norestart -allowlocalhttp \
  +ip 127.0.0.1 -port 27015 +sv_lan 1 +sv_hibernate_think 1 +net_usesocketsforloopback 1 -insecure +maxplayers 8 +map "${1:-gm_flatgrass}"
