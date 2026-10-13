//! In-memory `BlockCache` for `zcash_client_backend::sync::run`. The sync loop downloads a
//! batch, scans it, then deletes it, so memory stays bounded by the batch size.

use std::{ops::Range, sync::Mutex};

use async_trait::async_trait;
use zcash_client_backend::{
    data_api::{
        chain::{BlockCache, BlockSource, error},
        scanning::ScanRange,
    },
    proto::compact_formats::CompactBlock,
};
use zcash_protocol::consensus::BlockHeight;

#[derive(Default)]
pub struct MemBlockCache {
    blocks: Mutex<Vec<CompactBlock>>,
}

#[derive(Debug, thiserror::Error)]
#[error("block cache error")]
pub struct CacheError;

fn height_of(b: &CompactBlock) -> BlockHeight {
    BlockHeight::from_u32(b.height as u32)
}

impl BlockSource for MemBlockCache {
    type Error = CacheError;

    fn with_blocks<F, WalletErrT>(
        &self,
        from_height: Option<BlockHeight>,
        limit: Option<usize>,
        mut with_block: F,
    ) -> Result<(), error::Error<WalletErrT, Self::Error>>
    where
        F: FnMut(CompactBlock) -> Result<(), error::Error<WalletErrT, Self::Error>>,
    {
        let mut blocks: Vec<CompactBlock> = self
            .blocks
            .lock()
            .unwrap()
            .iter()
            .filter(|b| from_height.is_none_or(|h| height_of(b) >= h))
            .cloned()
            .collect();
        blocks.sort_by_key(|b| b.height);
        for b in blocks.into_iter().take(limit.unwrap_or(usize::MAX)) {
            with_block(b)?;
        }
        Ok(())
    }
}

#[async_trait]
impl BlockCache for MemBlockCache {
    fn get_tip_height(&self, range: Option<&ScanRange>) -> Result<Option<BlockHeight>, Self::Error> {
        let blocks = self.blocks.lock().unwrap();
        Ok(blocks
            .iter()
            .filter(|b| range.is_none_or(|r| r.block_range().contains(&height_of(b))))
            .map(height_of)
            .max())
    }

    async fn read(&self, range: &ScanRange) -> Result<Vec<CompactBlock>, Self::Error> {
        let mut out: Vec<CompactBlock> = self
            .blocks
            .lock()
            .unwrap()
            .iter()
            .filter(|b| range.block_range().contains(&height_of(b)))
            .cloned()
            .collect();
        out.sort_by_key(|b| b.height);
        Ok(out)
    }

    async fn insert(&self, mut compact_blocks: Vec<CompactBlock>) -> Result<(), Self::Error> {
        self.blocks.lock().unwrap().append(&mut compact_blocks);
        Ok(())
    }

    async fn delete(&self, range: ScanRange) -> Result<(), Self::Error> {
        let r: Range<BlockHeight> = range.block_range().clone();
        self.blocks.lock().unwrap().retain(|b| !r.contains(&height_of(b)));
        Ok(())
    }
}
