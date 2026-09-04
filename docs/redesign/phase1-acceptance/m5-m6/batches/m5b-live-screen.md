# M5b live TUI verification

## Initial screen
```text
Term4u local terminal  [1/1]  ctrl-t new  ctrl-w close  ctrl-q exit
           Term4u local terminal
          Type a command to begin.
ctrl-t new tab  ctrl-tab switch  ctrl-q exit
```

## Local command
```text
Term4u local terminal  [1/1]  ctrl-t new  ctrl-w close  ctrl-q exit
! printf 'M5B_LOCAL_OK\n'
M5B_LOCAL_OK
```

## New tab
```text
Term4u local terminal  [2/2]  ctrl-t new  ctrl-w close  ctrl-q exit
           Term4u local terminal
          Type a command to begin.
ctrl-t new tab  ctrl-tab switch  ctrl-q exit
```

## Ctrl-C interruption
```text
Term4u local terminal  [2/2]  ctrl-t new  ctrl-w close  ctrl-q exit
! sleep 30
^C
```
