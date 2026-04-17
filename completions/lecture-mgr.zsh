#compdef lecture-mgr
source <(COMPLETE=zsh lecture-mgr)
if [ "$funcstack[1]" = "_lecture-mgr" ]; then
  _clap_dynamic_completer_lecture_mgr "$@"
fi
