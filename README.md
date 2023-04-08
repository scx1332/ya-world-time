# ya-world-time
Library for keeping time as precise as possible 


## Variables

```
#provide list of time servers
YA_WORLD_TIME_SERVER_HOSTS=time.google.com;ntp.qix.ca;ntp.nict.jp;pool.ntp.org;time.cloudflare.com;ntp.fizyka.umk.pl;time.apple.com;time.fu-berlin.de;time.facebook.com
#max number of requests to send at once
YA_WORLD_TIME_MAX_AT_ONCE=50 
#max number of servers to process
YA_WORLD_TIME_MAX_TOTAL=100 
#max request timeout in milliseconds, otherwise drop request
YA_WORLD_TIME_MAX_TIMEOUT=300 
```

