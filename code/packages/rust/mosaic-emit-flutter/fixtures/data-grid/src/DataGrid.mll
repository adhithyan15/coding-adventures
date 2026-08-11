layout DataGrid {
  HostTable [ table ] {
    HostTableHead {
      Row {
        For ( each: slot: headers , as: header , index: column ) {
          Text ( content: ( header ) )
        }
      }
    }
    HostTableBody {
      For ( each: slot: rows , as: row , index: row-index ) {
        Row {
          For ( each: row , as: cell , index: column ) {
            Text ( content: ( cell ) )
          }
        }
      }
    }
  }
}
