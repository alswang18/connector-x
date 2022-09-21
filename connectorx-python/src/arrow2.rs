use crate::errors::ConnectorXPythonError;
use arrow2::{
    array::ArrayRef,
    chunk::Chunk,
    datatypes::{Field, Schema},
    ffi,
};
use connectorx::{
    destinations::arrow2::Arrow2Destination as ArrowDestination,
    prelude::*,
    sources::{
        mssql::MsSQLSource,
        mysql::{BinaryProtocol as MySQLBinaryProtocol, MySQLSource, TextProtocol},
        postgres::{
            rewrite_tls_args, BinaryProtocol as PgBinaryProtocol, CSVProtocol, CursorProtocol,
            SimpleProtocol, PostgresSource,
        },
        sqlite::SQLiteSource,
    },
    sql::CXQuery,
    transports::{
        MsSQLArrow2Transport as MsSQLArrowTransport, MySQLArrow2Transport as MySQLArrowTransport,
        OracleArrow2Transport as OracleArrowTransport,
        PostgresArrow2Transport as PostgresArrowTransport,
        SQLiteArrow2Transport as SQLiteArrowTransport,
    },
};
use fehler::throws;
use libc::uintptr_t;
use pyo3::prelude::*;
use pyo3::{PyAny, Python};
use std::sync::Arc;

#[throws(ConnectorXPythonError)]
pub fn write_arrow<'a>(
    py: Python<'a>,
    source_conn: &SourceConn,
    origin_query: Option<String>,
    queries: &[CXQuery<String>],
) -> &'a PyAny {
    let mut destination = ArrowDestination::new();

    // TODO: unlock gil if possible
    match source_conn.ty {
        SourceType::Postgres => {
            let (config, tls) = rewrite_tls_args(&source_conn.conn)?;
            debug!("Protocol: {}", protocol);
            match (protocol, tls) {
                ("csv", Some(tls_conn)) => {
                    let sb = PostgresSource::<CSVProtocol, MakeTlsConnector>::new(
                        config,
                        tls_conn,
                        queries.len(),
                    )?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<CSVProtocol, MakeTlsConnector>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("csv", None) => {
                    let sb =
                        PostgresSource::<CSVProtocol, NoTls>::new(config, NoTls, queries.len())?;
                    let dispatcher =
                        Dispatcher::<_, _, PostgresArrowTransport<CSVProtocol, NoTls>>::new(
                            sb,
                            &mut destination,
                            queries,
                            origin_query,
                        );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("binary", Some(tls_conn)) => {
                    let sb = PostgresSource::<PgBinaryProtocol, MakeTlsConnector>::new(
                        config,
                        tls_conn,
                        queries.len(),
                    )?;
                    let dispatcher =
                        Dispatcher::<
                            _,
                            _,
                            PostgresArrowTransport<PgBinaryProtocol, MakeTlsConnector>,
                        >::new(sb, &mut destination, queries, origin_query);
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("binary", None) => {
                    let sb = PostgresSource::<PgBinaryProtocol, NoTls>::new(
                        config,
                        NoTls,
                        queries.len(),
                    )?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<PgBinaryProtocol, NoTls>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("cursor", Some(tls_conn)) => {
                    let sb = PostgresSource::<CursorProtocol, MakeTlsConnector>::new(
                        config,
                        tls_conn,
                        queries.len(),
                    )?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<CursorProtocol, MakeTlsConnector>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("cursor", None) => {
                    let sb =
                        PostgresSource::<CursorProtocol, NoTls>::new(config, NoTls, queries.len())?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<CursorProtocol, NoTls>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("simple", Some(tls_conn)) => {
                    let sb = PostgresSource::<SimpleProtocol, MakeTlsConnector>::new(
                        config,
                        tls_conn,
                        queries.len(),
                    )?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<SimpleProtocol, MakeTlsConnector>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                ("simple", None) => {
                    let sb =
                        PostgresSource::<SimpleProtocol, NoTls>::new(config, NoTls, queries.len())?;
                    let dispatcher = Dispatcher::<
                        _,
                        _,
                        PostgresArrowTransport<SimpleProtocol, NoTls>,
                    >::new(
                        sb, &mut destination, queries, origin_query
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                _ => unimplemented!("{} protocol not supported", protocol),
            }
        }
        SourceType::SQLite => {
            // remove the first "sqlite://" manually since url.path is not correct for windows
            let path = &source_conn.conn.as_str()[9..];
            let source = SQLiteSource::new(path, queries.len())?;
            let dispatcher = Dispatcher::<_, _, SQLiteArrowTransport>::new(
                source,
                &mut destination,
                queries,
                origin_query,
            );
            debug!("Running dispatcher");
            dispatcher.run()?;
        }
        SourceType::MySQL => {
            debug!("Protocol: {}", protocol);
            match protocol {
                "binary" => {
                    let source = MySQLSource::<MySQLBinaryProtocol>::new(
                        &source_conn.conn[..],
                        queries.len(),
                    )?;
                    let dispatcher =
                        Dispatcher::<_, _, MySQLArrowTransport<MySQLBinaryProtocol>>::new(
                            source,
                            &mut destination,
                            queries,
                            origin_query,
                        );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                "text" => {
                    let source =
                        MySQLSource::<TextProtocol>::new(&source_conn.conn[..], queries.len())?;
                    let dispatcher = Dispatcher::<_, _, MySQLArrowTransport<TextProtocol>>::new(
                        source,
                        &mut destination,
                        queries,
                        origin_query,
                    );
                    debug!("Running dispatcher");
                    dispatcher.run()?;
                }
                _ => unimplemented!("{} protocol not supported", protocol),
            }
        }
        SourceType::MsSQL => {
            let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create runtime"));
            let source = MsSQLSource::new(rt, &source_conn.conn[..], queries.len())?;
            let dispatcher = Dispatcher::<_, _, MsSQLArrowTransport>::new(
                source,
                &mut destination,
                queries,
                origin_query,
            );
            debug!("Running dispatcher");
            dispatcher.run()?;
        }
        SourceType::Oracle => {
            let source = OracleSource::new(&source_conn.conn[..], queries.len())?;
            let dispatcher = Dispatcher::<_, _, OracleArrowTransport>::new(
                source,
                &mut destination,
                queries,
                origin_query,
            );
            debug!("Running dispatcher");
            dispatcher.run()?;
        }
        SourceType::BigQuery => {
            // TODO
        }
    }

    let (rbs, schema) = destination.arrow()?;
    let ptrs = to_ptrs(rbs, schema);
    let obj: PyObject = ptrs.into_py(py);
    obj.into_ref(py)
}

fn to_ptrs(
    rbs: Vec<Chunk<ArrayRef>>,
    schema: Arc<Schema>,
) -> (Vec<String>, Vec<Vec<(uintptr_t, uintptr_t)>>) {
    if rbs.is_empty() {
        return (vec![], vec![]);
    }

    let mut result = vec![];
    let names = schema.fields.iter().map(|f| f.name.clone()).collect();

    for rb in rbs {
        let mut cols = vec![];

        for array in rb.columns() {
            let array_ptr = Box::new(ffi::ArrowArray::empty());
            let schema_ptr = Box::new(ffi::ArrowSchema::empty());
            let array_ptr = Box::into_raw(array_ptr);
            let schema_ptr = Box::into_raw(schema_ptr);
            unsafe {
                ffi::export_field_to_c(
                    &Field::new("", array.data_type().clone(), true),
                    schema_ptr,
                );
                ffi::export_array_to_c(array.clone(), array_ptr);
            };
            cols.push((array_ptr as uintptr_t, schema_ptr as uintptr_t));
        }

        result.push(cols);
    }
    (names, result)
}
