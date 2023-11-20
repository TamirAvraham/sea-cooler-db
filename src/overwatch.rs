use std::collections::HashMap;




struct Overwatch<'a,T> {
    update_map:HashMap<String,Box<dyn FnMut(T)->()+'a+Send>>,
    delete_map:HashMap<String,Box<dyn FnMut(T)->()+'a+Send>>,
}

impl<'a,T> Overwatch<'a,T> {
    pub fn insert_update<F>(&mut self,key:&String,f:F) where F:FnMut(T)->()+'a+Send{
        self.update_map.insert(key.clone(), Box::new(f));
    }
    pub fn insert_delete<F>(&mut self,key:&String,f:F) where F:FnMut(T)->()+'a+Send{
        self.delete_map.insert(key.clone(), Box::new(f));
    }
    pub fn get_update(&mut self,key:&String,new_value:T){
        if let Some(f) = self.update_map.get_mut(key) {
            (f)(new_value);
        }
    }
    pub fn get_delete(&mut self,key:&String,new_value:T){
        if let Some(f) = self.delete_map.get_mut(key) {
            (f)(new_value);
        }
    }
    pub fn new()->Self{
        Self{
            update_map: HashMap::new(),
            delete_map: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_overwatch() {
        let mut overwatch = Overwatch::new();
        let key="key".to_string();
        let non_existent_key="key1".to_string();
        overwatch.insert_update(&key, |v| println!("v is {:?}",v));
        overwatch.insert_delete(&key, |v| println!("deleted v {:?}",v));
        overwatch.get_update(&key, "v1");
        overwatch.get_update(&key, "v2");
        overwatch.get_delete(&key, "v1");
        overwatch.get_update(&non_existent_key, "v3");
    }
    #[test]
    fn test_overwatch_value_capture() {
        let mut i=0;
        let key="k".to_string();
        let f=|v| {
            i+=1;
            println!("v is {} and this function has been called {} times",v,i);
        };

        let mut overwatch=Overwatch::new();
        overwatch.insert_update(&key, f);
        for ele in 0..5 {
            overwatch.get_update(&key, format!("test {}",ele));
        }
    }
}