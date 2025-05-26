use std::collections::HashMap;

pub fn persian_to_qwerty(input: &str) -> String {
    let map = persian_keymap();
    input
        .chars()
        .map(|ch| map.get(&ch).copied().unwrap_or(ch)) // translate if Persian, else keep original
        .collect()
}

fn persian_keymap() -> HashMap<char, char> {
    let mut map = HashMap::new();

    // Persian letters → QWERTY keys (based on physical key positions)
    map.insert('ض', 'q');
    map.insert('ص', 'w');
    map.insert('ث', 'e');
    map.insert('ق', 'r');
    map.insert('ف', 't');
    map.insert('غ', 'y');
    map.insert('ع', 'u');
    map.insert('ه', 'i');
    map.insert('خ', 'o');
    map.insert('ح', 'p');
    map.insert('ج', '[');
    map.insert('چ', ']');

    map.insert('ش', 'a');
    map.insert('س', 's');
    map.insert('ی', 'd');
    map.insert('ب', 'f');
    map.insert('ل', 'g');
    map.insert('ا', 'h');
    map.insert('ت', 'j');
    map.insert('ن', 'k');
    map.insert('م', 'l');
    map.insert('ک', ';');
    map.insert('گ', '\'');

    map.insert('ظ', 'z');
    map.insert('ط', 'x');
    map.insert('ز', 'c');
    map.insert('ر', 'v');
    map.insert('ذ', 'b');
    map.insert('د', 'n');
    map.insert('پ', 'm');
    map.insert('و', ',');
    map.insert('؟', '?');
    map.insert('،', ','); // Persian comma
                         // Persian digits → English digits
    map.insert('۰', '0');
    map.insert('۱', '1');
    map.insert('۲', '2');
    map.insert('۳', '3');
    map.insert('۴', '4');
    map.insert('۵', '5');
    map.insert('۶', '6');
    map.insert('۷', '7');
    map.insert('۸', '8');
    map.insert('۹', '9');


    map
}

