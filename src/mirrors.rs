use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use download_async::SocketAddrs;

use crate::downloader::download_file;
use crate::traits::{AsString,Error,ExpectUnwrap};
use futures::future::join_all;
use log::{error,trace};









