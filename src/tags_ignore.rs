use std::io::Write;



pub struct TagsIgnored {
    pub tags: Vec<String>
}

impl TagsIgnored {
    pub fn new(tags: Vec<String>) -> Self {
        Self {
            tags
        }
    }

    pub fn render(&self) -> Vec<String> {
        println!("Select the tags you want to ignore: ");

        for (i, tag) in self.tags.iter().enumerate() {
            println!("{}: {}", i, tag);
        }

        println!("Enter the numbers of the tags you want to ignore, separated by a comma: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        let mut ignored = Vec::new();

        for number in input.split(",") {
            let number = number.trim().parse::<usize>().unwrap();
            ignored.push(self.tags[number].clone());
        }

        ignored
    }
}