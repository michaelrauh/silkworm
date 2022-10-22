# silkworm
A library for append-only-database backed recursive search at scale

```mermaid
stateDiagram-v2
    [*] --> Abort_categorically?
    Abort_categorically? --> stop
    Abort_categorically? --> get_data
    get_data --> abort_data?
    abort_data? --> stop
    abort_data? --> get_friends
    get_friends --> abort_friends?
    abort_friends? --> stop 
    abort_friends? --> search 
    search --> save 
    save --> write 
    write --> stop
          ```  
