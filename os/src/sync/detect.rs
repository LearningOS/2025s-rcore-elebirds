//! Deadlock detection

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

/// Detectable trait
/// 
/// This trait is used to detect the deadlock of the system
pub trait Detectable {
    /// Get the available resource count
    fn get_avaliable(&self) -> usize;

    /// Get the owner of the allocated resource
    fn get_allocated(&self) -> Option<Vec<usize>>;

    /// Get the requester of the resource
    fn get_needed(&self) -> Option<Vec<usize>>; 

}

/// Detect the deadlock for a bunch of Detectable resources
pub fn detect(detectables: Vec<Arc<dyn Detectable>>, num_task: usize, mut adjust: impl FnMut(&mut Vec<usize>, &mut Vec<Vec<usize>>, &mut Vec<Vec<usize>>) -> ()) -> bool
{
    let num_resource = detectables.len();
    if num_resource == 0 { // 如果没有可检测的对象，直接true，即可用
        return true;
    }
    let mut available = vec![0; num_resource]; // 可用资源向量
    let mut allocated = vec![vec![0; num_resource]; num_task]; // 资源分配矩阵
    let mut needed = vec![vec![0; num_resource]; num_task]; // 资源需求矩阵
    for (did, detectable) in detectables.iter().enumerate() {
        available[did] = detectable.get_avaliable(); // 可用资源
        if let Some(allocated_list) = detectable.get_allocated() {
            for i in 0..allocated_list.len() {
                allocated[allocated_list[i]][did] += 1;
            }
        }
        if let Some(needed_list) = detectable.get_needed() {
            for i in 0..needed_list.len() {
                needed[needed_list[i]][did] += 1;
            }
        }
    }
    // 修正资源分配矩阵和需求矩阵(当前申请的资源需求数需要+1)
    adjust(&mut available, &mut allocated, &mut needed);
    // 调用伪银行家算法检测死锁
    pseudo_bankers_algorithm(&mut available, &mut allocated, &mut needed)
}

/// (伪)银行家算法, 用于检测死锁
/// 
/// 真正的银行家算法无法实现
fn pseudo_bankers_algorithm(
    available: &mut Vec<usize>,
    allocated: &mut Vec<Vec<usize>>,
    needed: &mut Vec<Vec<usize>>,
) -> bool {
    let num_resource = available.len();
    let num_task = allocated.len();
    let mut finish = vec![false; num_task];
    let mut work = available.clone();

    loop {
        let mut found = false;
        for i in 0..num_task {
            if !finish[i] && needed[i].iter().zip(work.iter()).all(|(n, w)| n <= w) {
                for j in 0..num_resource {
                    work[j] += allocated[i][j];
                }
                finish[i] = true;
                found = true;
            }
        }
        if !found {
            break;
        }
    }

    finish.iter().all(|&f| f)
}