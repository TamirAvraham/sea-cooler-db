use std::collections::hash_map::DefaultHasher;
use std::f64::consts::LN_2;
use std::hash::{Hash, Hasher};


const N: f64 = 4.0 * 1024.0 * 1024.0;
const FPR: f64 = 0.02;
const FPR_LN:f64=-3.9120230054;//ln.98
const  M:f64 = -(N * FPR_LN) / (LN_2 * LN_2);
const  K:f64=(M/ N) * LN_2;

pub struct BloomFilter {

    bit_array: Vec<bool>,
}

impl BloomFilter {
    fn new() -> BloomFilter {
        BloomFilter {
            bit_array: vec![false; M as usize],
        }
    }

    fn insert(&mut self,element:&str){
        (0..(K as usize)).for_each(|i|{
            let mut hasher = DefaultHasher::new();
            element.hash(&mut hasher);
            self.bit_array[(hasher.finish()%((M as u64)+i as u64)) as usize]=true;
        })
    }

    fn contains(&self,element:&str)->bool{
        (0..(K as usize)).all(|i|{
            let mut hasher = DefaultHasher::new();
            element.hash(&mut hasher);
            self.bit_array[(hasher.finish()%((M as u64)+i as u64)) as usize]
        })
    }
}
#[cfg(test)]
mod tests{
    use super::BloomFilter;

    #[test]
    fn test_bloom_filter(){
        let mut bf=BloomFilter::new();
        bf.insert("yosi1");
        bf.insert("yosi2");
        bf.insert("yosi3");

        assert!(bf.contains("yosi1"));
        assert!(bf.contains("yosi2"));
        assert!(bf.contains("yosi3"));
        assert!(!bf.contains("beni"));



    }
}