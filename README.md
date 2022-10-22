# silkworm
A library for append-only-database backed recursive search at scale

```mermaid
stateDiagram-v2
    [*] --> stop_categorically?
    stop_categorically? --> stop
    stop_categorically? --> get_data
    get_data --> stop_data?
    stop_data? --> stop
    stop_data? --> get_friends
    get_friends --> stop_friends?
    stop_friends? --> stop 
    stop_friends? --> search 
    search --> save 
    save --> write 
    write --> stop
```  
