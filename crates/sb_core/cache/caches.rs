// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;
use std::sync::Arc;

use crate::util::versions_util::deno;
use once_cell::sync::OnceCell;

use super::cache_db::CacheDB;
use super::cache_db::CacheDBConfiguration;
use super::check::TYPE_CHECK_CACHE_DB;
use super::deno_dir::DenoDirProvider;
use super::incremental::INCREMENTAL_CACHE_DB;
use super::node::NODE_ANALYSIS_CACHE_DB;
use super::parsed_source::PARSED_SOURCE_CACHE_DB;

pub struct Caches {
    dir_provider: Arc<DenoDirProvider>,
    fmt_incremental_cache_db: OnceCell<CacheDB>,
    lint_incremental_cache_db: OnceCell<CacheDB>,
    dep_analysis_db: OnceCell<CacheDB>,
    node_analysis_db: OnceCell<CacheDB>,
    type_checking_cache_db: OnceCell<CacheDB>,
}

impl Caches {
    pub fn new(dir: Arc<DenoDirProvider>) -> Self {
        Self {
            dir_provider: dir,
            fmt_incremental_cache_db: Default::default(),
            lint_incremental_cache_db: Default::default(),
            dep_analysis_db: Default::default(),
            node_analysis_db: Default::default(),
            type_checking_cache_db: Default::default(),
        }
    }

    fn make_db(
        cell: &OnceCell<CacheDB>,
        config: &'static CacheDBConfiguration,
        path: Option<PathBuf>,
    ) -> CacheDB {
        cell.get_or_init(|| {
            if let Some(path) = path {
                CacheDB::from_path(config, path, deno())
            } else {
                CacheDB::in_memory(config, deno())
            }
        })
        .clone()
    }

    pub fn fmt_incremental_cache_db(&self) -> CacheDB {
        Self::make_db(
            &self.fmt_incremental_cache_db,
            &INCREMENTAL_CACHE_DB,
            self.dir_provider
                .get_or_create()
                .ok()
                .map(|dir| dir.fmt_incremental_cache_db_file_path()),
        )
    }

    pub fn lint_incremental_cache_db(&self) -> CacheDB {
        Self::make_db(
            &self.lint_incremental_cache_db,
            &INCREMENTAL_CACHE_DB,
            self.dir_provider
                .get_or_create()
                .ok()
                .map(|dir| dir.lint_incremental_cache_db_file_path()),
        )
    }

    pub fn dep_analysis_db(&self) -> CacheDB {
        Self::make_db(
            &self.dep_analysis_db,
            &PARSED_SOURCE_CACHE_DB,
            self.dir_provider
                .get_or_create()
                .ok()
                .map(|dir| dir.dep_analysis_db_file_path()),
        )
    }

    pub fn node_analysis_db(&self) -> CacheDB {
        Self::make_db(
            &self.node_analysis_db,
            &NODE_ANALYSIS_CACHE_DB,
            self.dir_provider
                .get_or_create()
                .ok()
                .map(|dir| dir.node_analysis_db_file_path()),
        )
    }

    pub fn type_checking_cache_db(&self) -> CacheDB {
        Self::make_db(
            &self.type_checking_cache_db,
            &TYPE_CHECK_CACHE_DB,
            self.dir_provider
                .get_or_create()
                .ok()
                .map(|dir| dir.type_checking_cache_db_file_path()),
        )
    }
}
