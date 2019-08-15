/*
 * Copyright (c) 2019. The information/code/data contained within this file and all other files with the same copyright are protected under US Statutes. You must have explicit written access by Thomas P. Braun in order to access, view, modify, alter, or apply this code in any context commercial or non-commercial. If you have this code but were not given explicit written access by Thomas P. Braun, you must destroy the information herein for legal safety. You agree that if you apply the concepts herein without any written access, Thomas P. Braun will seek the maximum possible legal retribution.
 */

#[cfg(test)]
mod tests {
    use hyperbuf::partition_map::PartitionMap;
    use std::any::Any;

    #[test]
    fn test_partition_map() {
        let mut pm = PartitionMap::new();
        let pm = &mut pm;

            pm.store(0, 1000, i32::type_id(&0 as &i32));
            pm.store(1000, 1001, i64::type_id(&(0 as i64)));


        println!("{}", pm);
    }
}