# Counters stats rs

Crate to support high performance debug counters for heavy multithreaded applications
All threads write to a thread local counter, then when the counters are requested
an aggregation across all the variables is performed
