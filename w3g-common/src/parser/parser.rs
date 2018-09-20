use byteorder::{ReadBytesExt, LittleEndian};

use std::fs::File;

use std::collections::VecDeque;
 
use std::io::{Cursor, Read}; 

use libflate::zlib::Decoder;

use ::errors::*;


/// Size of '\0' that commonly occurs at the end of a String
const NULL_BYTE_LENGTH: usize = 1;

pub fn parse_replay(raw: &mut Read) -> Result<Replay>
{
    let magic_string = extract_fixed_length_string(raw, 28)?;
    let file_offset = extract_unsigned_dword(raw)?;
    let compressed_size = extract_unsigned_dword(raw)?;
    let header_version = extract_unsigned_dword(raw)?;
    let decompressed_size = extract_unsigned_dword(raw)?;
    let number_of_compressed_blocks = extract_unsigned_dword(raw)?;

    let replay_header = extract_replay_header(raw)?;
    let mut stream = ReplayStream::from_file(raw);

    let game_header = stream.extract_game_header()?;
    let replay_blocks = stream.extract_blocks()?;
    
    Ok(
        Replay
        { 
            magic_string,
            file_offset,
            compressed_size, 
            header_version,
            decompressed_size, 
            number_of_compressed_blocks,

            replay_header,
            game_header,
            replay_blocks,
        }
    )
}

pub fn extract_replay(path: &str) -> Result<Replay>
{
    let mut file = File::open(path)?;
    
    parse_replay(&mut file)
}

fn extract_replay_header(file: &mut Read) -> Result<ReplayHeader>
{
    Ok(
        ReplayHeader {
            version_string: extract_fixed_length_string(file, 4)?,
            version_number: extract_unsigned_dword(file)?,
            build_number: extract_unsigned_word(file)?,
            flags: extract_unsigned_word(file)?,
            duration: extract_unsigned_dword(file)?,
            crc32: extract_unsigned_dword(file)?,
        }
    )
}

fn extract_unsigned_word(file: &mut Read) -> Result<u16>
{
    let mut buffer = vec![0u8; 2];
    file.read_exact(&mut buffer)?;

    Ok(Cursor::new(buffer).read_u16::<LittleEndian>()?)
}

fn extract_unsigned_dword(file: &mut Read) -> Result<u32>
{
    let mut buffer = vec![0u8; 4];
    file.read_exact(&mut buffer)?;

    Ok(Cursor::new(buffer).read_u32::<LittleEndian>()?)
}

fn extract_fixed_length_string(file: &mut Read, length: usize) -> Result<String>
{
    let mut buffer = vec![0u8; length];
    file.read_exact(&mut buffer)?;
        
    Ok(String::from_utf8(buffer)?)
}

struct ReplayStream<R: Read>
{
    raw_file: R,
    decompressed_bytes: VecDeque<u8>,
}

impl<R: Read> ReplayStream<R>
{
    fn from_file(file: R) -> ReplayStream<R>
    {
        ReplayStream
        {
            raw_file: file,
            decompressed_bytes: VecDeque::new(),
        }
    }

    fn decompress_data(&mut self) -> Result<()>
    {
        let compressed_size = extract_unsigned_word(&mut self.raw_file)? as usize;
        let decompressed_size = extract_unsigned_word(&mut self.raw_file)? as usize;
        let _crc32 = extract_unsigned_dword(&mut self.raw_file)?;

        let mut compressed_data = vec![0u8; compressed_size];
        self.raw_file.read_exact(&mut compressed_data)?;
        
        let mut decompressed_data = Vec::with_capacity(decompressed_size);

        let mut decoder = Decoder::new(Cursor::new(compressed_data))?;
        decoder.read_to_end(&mut decompressed_data)?;

        for x in decompressed_data
        {
            self.decompressed_bytes.push_back(x);
        }

        Ok(())
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>>
    {
        let mut buffer = Vec::with_capacity(length);

        for _ in 0..length
        {
            if self.decompressed_bytes.len() < 1
            {
                self.decompress_data()?;
            }

            buffer.push(self.decompressed_bytes.pop_front()
                .ok_or("no data left in decompressed bytes")?);
        }

        Ok(buffer)
    }

    fn read_float32(&mut self) -> Result<f32>
    {
        Ok(Cursor::new(self.read_bytes(4)?).read_f32::<LittleEndian>()?)
    }

    fn read_signed_dword(&mut self) -> Result<i32>
    {
        Ok(Cursor::new(self.read_bytes(4)?).read_i32::<LittleEndian>()?)
    }

    fn read_unsigned_dword(&mut self) -> Result<u32>
    {
        Ok(Cursor::new(self.read_bytes(4)?).read_u32::<LittleEndian>()?)
    }

    fn read_unsigned_qword(&mut self) -> Result<u64>
    {
        Ok(Cursor::new(self.read_bytes(8)?).read_u64::<LittleEndian>()?)
    }

    fn read_unsigned_word(&mut self) -> Result<u16>
    {
        Ok(Cursor::new(self.read_bytes(2)?).read_u16::<LittleEndian>()?)
    }

    fn read_unsigned_byte(&mut self) -> Result<u8>
    {
        Ok(Cursor::new(self.read_bytes(1)?).read_u8()?)
    }

    fn read_null_terminated_string(&mut self) -> Result<String>
    {
        let mut buffer = vec!();
        
        let mut last_byte = self.read_unsigned_byte()?;
        while last_byte != 0b0
        {
            buffer.push(last_byte);
            last_byte = self.read_unsigned_byte()?;
        }
        
        Ok(String::from_utf8(buffer)?)
    }

    fn read_encoded_string(&mut self) -> Result<Vec<u8>>
    {
        let mut buffer = vec!();
        
        let mut last_byte = self.read_unsigned_byte()?;
        while last_byte != 0b0
        {
            buffer.push(last_byte);
            last_byte = self.read_unsigned_byte()?;
        }
        buffer.push(last_byte);

        Ok(buffer)
    }

    fn extract_game_header(&mut self) -> Result<GameHeader>
    {
        let unknown = self.read_unsigned_dword()?;
        let replay_saver = self.extract_player_record(None)?;
        let game_name = self.read_null_terminated_string()?;
        let _null_byte = self.read_bytes(1);
        let encoded_string = self.read_encoded_string()?;
        let number_of_players = self.read_unsigned_dword()?;
        let game_type = self.read_unsigned_dword()?;
        let language_id = self.read_unsigned_dword()?;
        
        let mut players = Vec::with_capacity(number_of_players as usize);
        let mut record_id = self.read_unsigned_byte()?;
        while record_id == 0x16
        {
            players.push(self.extract_player_record(Some(record_id))?);
            let _ = self.read_unsigned_dword()?;

            record_id = self.read_unsigned_byte()?;
        }

        let game_record = self.extract_game_record(Some(record_id))?;

        Ok(
            GameHeader { 
                unknown,
                replay_saver,
                game_name,
                encoded_string,
                number_of_players,
                game_type,
                language_id,
                players,
                game_record,
            }
        )
    }

    fn extract_player_record(&mut self, record_id: Option<u8>) -> Result<PlayerRecord>
    {
        let record_id = match record_id {
            Some(id) => id,
            None => self.read_unsigned_byte()?,
        };
        let player_id = self.read_unsigned_byte()?;
        let player_name = self.read_null_terminated_string()?;
        let additional_data_size = self.read_unsigned_byte()?;
        let additional_data = self.read_bytes(additional_data_size as usize)?;
        
        Ok(
            PlayerRecord {
                record_id,
                player_id,
                player_name,
                additional_data_size,
                additional_data,
            }
        )
    }

    fn extract_unit_inventory(&mut self) -> Result<Vec<UnitInventory>>
    {
        let inventory_size = self.read_unsigned_dword()?;
        let mut inventory = Vec::with_capacity(inventory_size as usize);
        for _ in 0..inventory_size
        {
            inventory.push(UnitInventory::new(self.read_unsigned_dword()?, self.read_unsigned_dword()?, self.read_unsigned_dword()?));
        }

        Ok(inventory)
    }

    fn extract_unit_abilites(&mut self) -> Result<Vec<UnitAbility>>
    {
        let abilities_size = self.read_unsigned_dword()?;
        let mut abilities = Vec::with_capacity(abilities_size as usize);
        for _ in 0..abilities_size
        {
            abilities.push(UnitAbility::new(self.read_unsigned_dword()?, self.read_unsigned_dword()?));
        }

        Ok(abilities)
    }

    fn extract_game_record(&mut self, record_id: Option<u8>) -> Result<GameRecord>
    {
        let record_id = match record_id {
            Some(id) => id,
            None => self.read_unsigned_byte()?,
        };
        let num_data_bytes = self.read_unsigned_word()?;
        let num_slot_records = self.read_unsigned_byte()?;
        let mut slot_records = Vec::with_capacity(num_slot_records as usize);
        for _ in 0..num_slot_records
        {
            slot_records.push(self.extract_slot_record()?);
        }
        let random_seed = self.read_unsigned_dword()?;
        let select_mode = self.read_unsigned_byte()?;
        let start_spot_count = self.read_unsigned_byte()?;

        Ok( 
            GameRecord {
                record_id,
                num_data_bytes, 
                num_slot_records,
                slot_records, 
                random_seed,
                select_mode,
                start_spot_count,
            }
        )
    }

    fn extract_slot_record(&mut self) -> Result<SlotRecord>
    {
        Ok(
            SlotRecord { 
                player_id: self.read_unsigned_byte()?, 
                download_percent: self.read_unsigned_byte()?,
                slot_status: self.read_unsigned_byte()?,
                player_flag: self.read_unsigned_byte()?,
                team_number: self.read_unsigned_byte()?,
                color: self.read_unsigned_byte()?,
                race: self.read_unsigned_byte()?,
                ai_strength: self.read_unsigned_byte()?,
                handicap: self.read_unsigned_byte()?,
            }
        )
    }

    fn extract_blocks(&mut self) -> Result<Vec<ReplayBlock>>
    {
        let mut blocks = Vec::new();

        let mut block_id = self.read_unsigned_byte()?;
        while block_id != 0x0
        {

            match block_id 
            {
                0x17 => 
                {
                    blocks.push(
                        ReplayBlock::LeaveGame { 
                            reason: self.read_unsigned_dword()?, 
                            player_id: self.read_unsigned_byte()?, 
                            result: self.read_unsigned_dword()?, 
                            session_leave_count: self.read_unsigned_dword()?,
                        }
                    );
                },
                0x1A =>
                {
                    blocks.push(
                        ReplayBlock::LoadStarted1 { 
                            unknown: self.read_unsigned_dword()?, 
                        }
                    );
                },
                0x1B =>
                {
                    blocks.push(
                        ReplayBlock::LoadStarted2 { 
                            unknown: self.read_unsigned_dword()?, 
                        }
                    );
                },
                0x1C =>
                {
                    blocks.push(
                        ReplayBlock::GameStarted { 
                            unknown: self.read_unsigned_dword()?, 
                        }
                    );
                },
                0x1E =>
                {
                    let num_bytes = self.read_unsigned_word()?;
                    let time_increment = self.read_unsigned_word()?;
                    // minus 2 because 2 for time_increment
                    let commands = self.extract_commands((num_bytes - 2) as usize)?;

                    blocks.push(
                        ReplayBlock::TickPreOverflow { 
                            num_bytes,
                            time_increment,
                            commands,
                        }
                    );
                }
                0x1F =>
                {
                    let num_bytes = self.read_unsigned_word()?;
                    let time_increment = self.read_unsigned_word()?;
                    // minus 2 because 2 for time_increment
                    let commands = self.extract_commands((num_bytes - 2) as usize)?;

                    blocks.push(
                        ReplayBlock::Tick { 
                            num_bytes,
                            time_increment,
                            commands,
                        }
                    );
                }
                0x20 =>
                {
                    let player_id = self.read_unsigned_byte()?;
                    let num_bytes = self.read_unsigned_word()?;
                    let flags = self.read_unsigned_byte()?;
                    let chat_mode = self.read_unsigned_dword()?;
                    // minus 6 because 1 for flags, 4 for chat_mode, 1 for '\0'
                    let message = String::from_utf8(self.read_bytes((num_bytes as usize) - 6)?)?;

                    let ending_byte = self.read_unsigned_byte()?;
                    if ending_byte != 0x0
                    {
                        bail!(String::from("String did not end in \0"));
                    }

                    blocks.push(
                        ReplayBlock::PlayerChat { 
                            player_id,
                            num_bytes,
                            flags,
                            chat_mode,
                            message,
                        }
                    );
                },
                0x22 =>
                {
                    blocks.push(
                        ReplayBlock::RandomSeed { 
                            num_bytes: self.read_unsigned_byte()?, 
                            unknown: self.read_unsigned_dword()?,
                        }
                    );
                },
                0x23 =>
                {
                    blocks.push(
                        ReplayBlock::Desync {  
                            tick_count: self.read_unsigned_dword()?,  
                            checksum: self.read_unsigned_dword()?, 
                            remaining_players: self.read_unsigned_byte()?,
                        }
                    );
                },
                0x2F =>
                {
                    blocks.push(
                        ReplayBlock::ForceGameEndCountdown {  
                            mode: self.read_unsigned_dword()?,  
                            time: self.read_unsigned_dword()?,  
                        }
                    );
                },
                _ => {},
            }

            
            block_id = self.read_unsigned_byte()?;
        }
        
        Ok(blocks)
    }

    fn extract_commands(&mut self, commands_size: usize) -> Result<Vec<Command>>
    {
        let mut commands = Vec::new();

        let mut bytes_read = 0;
        while bytes_read < commands_size
        {
            let player_id = self.read_unsigned_byte()?;
            let num_bytes = self.read_unsigned_word()?;
            let actions = self.extract_actions(num_bytes as usize)?;

            commands.push( Command {
                player_id,
                num_bytes,
                actions,
            });
            bytes_read = bytes_read + 3 + (num_bytes as usize);
        }

        Ok(commands) 
    }

    fn extract_actions(&mut self, actions_size: usize) -> Result<Vec<Action>>
    {
        let mut actions = Vec::new();

        let mut bytes_read = 0;
        while bytes_read < actions_size
        {
            let action_id = self.read_unsigned_byte()?;
            bytes_read = bytes_read + 1; 

            match action_id
            {
                0x01 =>
                { 
                    actions.push(
                        Action::PauseGame {}
                    );
                },
                0x02 =>
                { 
                    actions.push(
                        Action::ResumeGame {}
                    );
                },
                0x03 =>
                { 
                    let speed = GameSpeed::from_u8(self.read_unsigned_byte()?)?;
                    bytes_read = bytes_read + 1;

                    actions.push(
                        Action::SetGameSpeed 
                        {
                            speed,
                        }
                    );
                },
                0x04 =>
                { 
                    actions.push(
                        Action::IncreaseGameSpeed {}
                    );
                },
                0x05 =>
                { 
                    actions.push(
                        Action::DecreaseGameSpeed {}
                    );
                },
                0x06 =>
                { 
                    let game_name = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + game_name.len() + NULL_BYTE_LENGTH;
                    actions.push(
                        Action::SaveGame 
                        {
                            game_name,
                        }
                    );
                },
                0x07 =>
                { 
                    bytes_read = bytes_read + 4;

                    actions.push(
                        Action::SaveGameFinish 
                        {
                            unknown: self.read_unsigned_dword()?,
                        }
                    );
                },
                0x10 =>
                { 
                    let flags = OrderType::from_u16(self.read_unsigned_word()?)?;
                    let order_id = self.read_unsigned_dword()?;
                    let unknown = self.extract_game_object()?;

                    bytes_read = bytes_read + 2 + 3*4;

                    actions.push(
                        Action::SelfOrder 
                        {
                            flags,
                            order_id,
                            unknown,
                        }
                    );
                },
                0x11 =>
                {
                    let flags = OrderType::from_u16(self.read_unsigned_word()?)?;
                    let order_id = self.read_unsigned_dword()?;
                    let unknown = self.extract_game_object()?;
                    let x = self.read_float32()?;
                    let y = self.read_float32()?;

                    bytes_read = bytes_read + 2 + 5*4;

                    actions.push(
                        Action::PointOrder 
                        {
                            flags,
                            order_id,
                            unknown,
                            x,
                            y,
                        }
                    );
                },
                0x12 =>
                { 
                    let flags = OrderType::from_u16(self.read_unsigned_word()?)?;
                    let order_id = self.read_unsigned_dword()?;
                    let unknown =self.extract_game_object()?;
                    let x = self.read_float32()?;
                    let y = self.read_float32()?;
                    let target = self.extract_game_object()?;

                    bytes_read = bytes_read + 2 + 7*4;

                    actions.push(
                        Action::ObjectOrder 
                        {
                            flags,
                            order_id,
                            unknown,
                            x,
                            y,
                            target,
                        }
                    );
                },
                0x13 =>
                { 
                    let flags = OrderType::from_u16(self.read_unsigned_word()?)?;
                    let order_id = self.read_unsigned_dword()?; 
                    let unknown = self.extract_game_object()?;
                    let x = self.read_float32()?;
                    let y = self.read_float32()?;
                    let receiver = self.extract_game_object()?;
                    let item = self.extract_game_object()?;
 

                    bytes_read = bytes_read + 2 + 9*4;

                    actions.push(
                        Action::DropOrGiveItem 
                        {
                            flags,
                            order_id,
                            unknown,
                            x,
                            y,
                            receiver,
                            item,
                        }
                    );
                },
                0x14 =>
                { 
                    let flags = OrderType::from_u16(self.read_unsigned_word()?)?;
                    let order_id = self.read_unsigned_dword()?; 
                    let unknown = self.extract_game_object()?;
                    let x = self.read_float32()?;
                    let y = self.read_float32()?;
                    let target_type = self.read_unsigned_dword()?;
                    let target_flags = self.read_unsigned_qword()?;
                    let target_owner = self.read_unsigned_byte()?;
                    let target_x = self.read_float32()?;
                    let target_y = self.read_float32()?;
 
                    bytes_read = bytes_read + 2 + 8*4 + 9;

                    actions.push(
                        Action::FogObjectOrder 
                        {
                            flags,
                            order_id,
                            unknown,
                            x,
                            y,
                            target_type,
                            target_flags,
                            target_owner,
                            target_x,
                            target_y, 
                        }
                    );
                },
                0x16 =>
                { 
                    let select_mode = SelectionOperation::from_u8(self.read_unsigned_byte()?)?;
                    let num_targets = self.read_unsigned_word()?;
                    let mut targets = Vec::with_capacity(num_targets as usize);
                    for _ in 0..num_targets
                    {
                        targets.push(self.extract_game_object()?);
                    }

                    bytes_read = bytes_read + 1 + 2 + (num_targets as usize)*(2*4);

                    actions.push(
                        Action::ChangeSelection 
                        {
                            select_mode,
                            targets,
                        }
                    );
                },
                0x17 =>
                { 
                    let group_number = self.read_unsigned_byte()?;
                    let num_targets = self.read_unsigned_word()?;
                    let mut targets = Vec::with_capacity(num_targets as usize);
                    for _ in 0..num_targets
                    {
                        targets.push(self.extract_game_object()?);
                    }

                    bytes_read = bytes_read + 1 + 2 + (num_targets as usize)*(2*4);

                    actions.push(
                        Action::AssignGroup 
                        {
                            group_number,
                            targets,
                        }
                    );
                },
                0x18 =>
                { 
                    bytes_read = bytes_read + 2*1;

                    actions.push(
                        Action::SelectGroup 
                        {
                            group_number: self.read_unsigned_byte()?, 
                            unknown: self.read_unsigned_byte()?,  
                        }
                    );
                },
                0x19 =>
                { 
                    bytes_read = bytes_read + 3*4;

                    actions.push(
                        Action::SelectSubGroup 
                        {
                            item_id: self.read_unsigned_dword()?, 
                            target: self.extract_game_object()?, 
                        }
                    );
                },
                0x1A =>
                { 
                    actions.push(
                        Action::PreSubSelection {}
                    );
                },
                0x1B =>
                { 
                    bytes_read = bytes_read + 1 + 2*4;

                    actions.push(
                        Action::TriggerSelectionEvent 
                        {
                            operation: SelectionOperation::from_u8(self.read_unsigned_byte()?)?,
                            target: self.extract_game_object()?,
                        }
                    );
                },
                0x1C =>
                { 
                    bytes_read = bytes_read + 1 + 2*4;

                    actions.push(
                        Action::SelectGroundItem 
                        {
                            flags: self.read_unsigned_byte()?, 
                            target: self.extract_game_object()?,
                        }
                    );
                },
                0x1D =>
                { 
                    bytes_read = bytes_read + 2*4;

                    actions.push(
                        Action::CancelHeroRevival 
                        { 
                            target: self.extract_game_object()?, 
                        }
                    );
                },
                0x1E =>
                { 
                    bytes_read = bytes_read + 1 + 4;

                    actions.push(
                        Action::CancelUnitInQueue 
                        { 
                            slot_index: self.read_unsigned_byte()?,  
                            unit_id: self.read_unsigned_dword()?,  
                        }
                    );
                },
                0x21 =>
                { 
                    bytes_read = bytes_read + 2*4;

                    actions.push(
                        Action::Unknown21 
                        { 
                            unknown_a: self.read_unsigned_dword()?,  
                            unknown_b: self.read_unsigned_dword()?,  
                        }
                    );
                },
                0x20 =>
                { 
                    actions.push(
                        Action::CheatTheDudeAbides {}
                    );
                },
                0x22 =>
                { 
                    actions.push(
                        Action::CheatSomebodySetUpUsTheBomb {}
                    );
                },
                0x23 =>
                { 
                    actions.push(
                        Action::CheatWarpTen {}
                    );
                },
                0x24 =>
                { 
                    actions.push(
                        Action::CheatIocainePowder {}
                    );
                },
                0x25 =>
                { 
                    actions.push(
                        Action::CheatPointBreak {}
                    );
                },
                0x26 =>
                { 
                    actions.push(
                        Action::CheatWhosYourDaddy {}
                    );
                },
                0x27 =>
                { 
                    bytes_read = bytes_read + 1 + 4;

                    actions.push(
                        Action::CheatKeyserSoze {
                            unknown: self.read_unsigned_byte()?,
                            gold: self.read_signed_dword()?,
                        }
                    );
                },
                0x28 =>
                { 
                    bytes_read = bytes_read + 1 + 4;

                    actions.push(
                        Action::CheatLeafItToMe {
                            unknown: self.read_unsigned_byte()?,
                            lumber: self.read_signed_dword()?,
                        }
                    );
                },
                0x29 =>
                { 
                    actions.push(
                        Action::CheatThereIsNoSpoon {}
                    );
                },
                0x2A=>
                { 
                    actions.push(
                        Action::CheatStrengthAndHonor {}
                    );
                },
                0x2B=>
                { 
                    actions.push(
                        Action::CheatItVexesMe {}
                    );
                },
                0x2C=>
                { 
                    actions.push(
                        Action::CheatWhoIsJohnGalt {}
                    );
                },
                0x2D =>
                { 
                    bytes_read = bytes_read + 1 + 4;

                    actions.push(
                        Action::CheatGreedIsGood {
                            unknown: self.read_unsigned_byte()?,
                            resources: self.read_signed_dword()?,
                        }
                    );
                },
                0x2E =>
                { 
                    bytes_read = bytes_read + 4;

                    actions.push(
                        Action::CheatDaylightSavings {
                            time: self.read_float32()?,
                        }
                    );
                },
                0x2F =>
                { 
                    actions.push(
                        Action::CheatISeeDeadPeople {}
                    );
                },
                0x30 =>
                { 
                    actions.push(
                        Action::CheatSynergy {}
                    );
                },
                0x31 =>
                { 
                    actions.push(
                        Action::CheatSharpAndShiny {}
                    );
                },
                0x32 =>
                { 
                    actions.push(
                        Action::CheatAllYourBaseAreBelongToUs {}
                    );
                },
                0x50 =>
                { 
                    bytes_read = bytes_read + 1 + 4;

                    actions.push(
                        Action::ChangeAlly {
                            player_id: self.read_unsigned_byte()?,
                            flags: AllianceType::from_u32(self.read_unsigned_dword()?)?, 
                        }
                    );
                },
                0x51 =>
                { 
                    bytes_read = bytes_read + 1 + 4 + 4;

                    actions.push(
                        Action::TransferResources {
                            player_id: self.read_unsigned_byte()?,
                            gold_transfered: self.read_signed_dword()?,
                            lumber_transfered: self.read_signed_dword()?,
                        }
                    );
                },
                0x60 =>
                { 
                    let event = self.extract_game_object()?; 
                    let message = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + 2*4 + message.len() + NULL_BYTE_LENGTH;

                    actions.push(
                        Action::MapTriggerChat { 
                            event,
                            message,
                        }
                    );
                },
                0x61 =>
                { 
                    actions.push(
                        Action::Esc {}
                    );
                },
                0x62 =>
                { 
                    let thread = self.extract_game_object()?;
                    let wait_count = self.read_unsigned_dword()?;

                    bytes_read = bytes_read + 3*4;

                    actions.push(
                        Action::TriggerSleepOrSyncFinished { 
                            thread,
                            wait_count,
                        }
                    );
                },
                0x63 =>
                { 
                    let thread = self.extract_game_object()?;

                    bytes_read = bytes_read + 2*4;

                    actions.push(
                        Action::TriggerSyncReady { 
                            thread,
                        }
                    );
                },
                0x64 =>
                {
                    let trackable = self.extract_game_object()?;

                    bytes_read = bytes_read + 2*4;

                    actions.push(
                        Action::TriggerMouseClickedTrackable { 
                            trackable,
                        }
                    );
                },
                0x65 =>
                {
                    let trackable = self.extract_game_object()?;

                    bytes_read = bytes_read + 2*4;

                    actions.push(
                        Action::TriggerMouseTouchedTrackable { 
                            trackable,
                        }
                    );
                },
                0x66 =>
                { 
                    actions.push(
                        Action::EnterHeroSkillSubMenu {}
                    );
                },
                0x67 =>
                { 
                    actions.push(
                        Action::EnterBuildingSubMenu {}
                    );
                },
                0x68 =>
                {
                    bytes_read = bytes_read + 3*4;

                    actions.push(
                        Action::MiniMapSignal { 
                            location_x: self.read_float32()?,
                            location_y: self.read_float32()?,
                            duration: self.read_float32()?,  
                        }
                    );
                },
                0x69 =>
                {
                    let dialog = self.extract_game_object()?;
                    let button = self.extract_game_object()?;

                    bytes_read = bytes_read + 4*4;

                    actions.push(
                        Action::DialogButtonClicked { 
                            dialog,
                            button,
                        }
                    );
                },
                0x6A =>
                {
                    let button = self.extract_game_object()?;
                    let dialog = self.extract_game_object()?;
                    
                    bytes_read = bytes_read + 4*4;

                    actions.push(
                        Action::DialogAnyButtonClicked { 
                            button,
                            dialog,
                        }
                    );
                },
                0x6B =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;
                    let value = self.read_signed_dword()?;

                    bytes_read = bytes_read + 4 + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncStoredInteger {
                            file,
                            group,
                            key,
                            value,
                        }
                    );
                },
                0x6C =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + 4 + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncStoredFloat {
                            file,
                            group,
                            key,
                            value: self.read_float32()?,
                        }
                    );
                },
                0x6D =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncStoredBoolean {
                            file,
                            group,
                            key,
                            value: self.read_unsigned_dword()?,
                        }
                    );
                },
                0x6E =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;
                    let unit_type = self.read_unsigned_dword()?;
                    let inventory = self.extract_unit_inventory()?;
                    let experience = self.read_unsigned_dword()?;
                    let level_ups = self.read_unsigned_dword()?;
                    let skill_points = self.read_unsigned_dword()?;
                    let proper_name_index = self.read_unsigned_word()?;
                    let unknown1 = self.read_unsigned_word()?;
                    let base_strength = self.read_unsigned_dword()?;
                    let bonus_strength_per_level = self.read_float32()?;
                    let base_agility = self.read_unsigned_dword()?;
                    let bonus_move_speed = self.read_float32()?;
                    let bonus_attack_speed = self.read_float32()?;
                    let bonus_agility_per_level = self.read_float32()?;
                    let base_intelligence = self.read_unsigned_dword()?;
                    let bonus_intelligence_per_level = self.read_float32()?;
                    let abilities = self.extract_unit_abilites()?;
                    let bonus_health = self.read_float32()?;
                    let bonus_mana = self.read_float32()?;
                    let sight_radius_day = self.read_float32()?;
                    let unknown2 = self.read_unsigned_dword()?;
                    let unknown3 = self.read_unsigned_dword()?;
                    let unknown4 = self.read_unsigned_dword()?;
                    let unknown5 = self.read_unsigned_dword()?;
                    let hotkey_flags = self.read_unsigned_word()?;

                    bytes_read = bytes_read 
                        + file.len() 
                        + group.len() 
                        + key.len() 
                        + 3*NULL_BYTE_LENGTH
                        + 4
                        + inventory.len() * 3*4 + 4
                        + 3*4
                        + 2*2
                        + 8*4
                        + abilities.len() * 2*4 + 4
                        + 7*4
                        + 2;

                    actions.push(
                        Action::SyncStoredUnit {
                            file,
                            group,
                            key,
                            unit_type,
                            inventory,
                            experience,
                            level_ups,
                            skill_points,
                            proper_name_index,
                            unknown1,
                            base_strength,
                            bonus_strength_per_level,
                            base_agility,
                            bonus_move_speed,
                            bonus_attack_speed,
                            bonus_agility_per_level,
                            base_intelligence,
                            bonus_intelligence_per_level,
                            abilities,
                            bonus_health,
                            bonus_mana,
                            sight_radius_day,
                            unknown2,
                            unknown3,
                            unknown4,
                            unknown5,
                            hotkey_flags,
                        }
                    );
                },
                0x6F =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;
                    let value = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 4*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncStoredString {
                            file,
                            group,
                            key,
                            value,
                        }
                    );
                },
                0x70 =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncEmptyInteger {
                            file,
                            group,
                            key,
                        }
                    );
                },
                0x71 =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?; 

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncEmptyString {
                            file,
                            group,
                            key,
                        }
                    );
                },
                0x72 =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncEmptyBoolean {
                            file,
                            group,
                            key,
                        }
                    );
                },
                0x73 =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncEmptyUnit {
                            file,
                            group,
                            key,
                        }
                    );
                },
                0x74 =>
                {
                    let file = self.read_null_terminated_string()?;
                    let group = self.read_null_terminated_string()?;
                    let key = self.read_null_terminated_string()?;

                    bytes_read = bytes_read + file.len() + group.len() + key.len() + 3*NULL_BYTE_LENGTH;

                    actions.push(
                        Action::SyncEmptyFloat {
                            file,
                            group,
                            key,
                        }
                    );
                },
                0x75 =>
                {
                    bytes_read = bytes_read + 1;

                    actions.push(
                        Action::TriggerArrow {
                            key: ArrowKeyEvent::from_u8(self.read_unsigned_byte()?)?,
                        }
                    );
                },
                _ => bail!(format!("Unknown action_id: {}", action_id)),
            };
        }

        Ok(actions)
    }

    fn extract_game_object(&mut self) -> Result<GameObject>
    {
        Ok(GameObject::new(self.read_unsigned_dword()?, self.read_unsigned_dword()?)) 
    }

}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Replay
{
    /* 28 characters to confirm this file should be parsed. ends in 0x1A, 0x00 */
    pub magic_string: String, 

    /* Specifies how long the header is, 1 dword 0x40 <= v1.06, 0x44 for >= 1.07 */
    pub file_offset: u32,

    /* 1dword */
    pub compressed_size: u32,

    /* 1 dword 0x00 <= v1.06, 0x01 >= v1.07 */
    pub header_version: u32,

    /* 1 dword */
    pub decompressed_size: u32,

    /* 1 dword */
    pub number_of_compressed_blocks: u32,

    /* Stores stats about the Replay */
    pub replay_header: ReplayHeader,

    pub game_header: GameHeader,

    pub replay_blocks: Vec<ReplayBlock>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplayHeader {
    /* 1 dword */
    pub version_string: String,

    /* 1 dword */
    pub version_number: u32,

    /* 1 word */
    pub build_number: u16,

    /* 1 word */
    pub flags: u16,

    /* 1 dword (milliseconds) */
    pub duration: u32,

    /* 1 dword, includes itself in the computation but uses 0 for the field's value */
    pub crc32: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GameHeader {
    /* 1 dword */
    pub unknown: u32,
    /* The guy whose POV the replay is from */
    pub replay_saver: PlayerRecord,

    pub game_name: String,

    pub encoded_string: Vec<u8>,
    /* 1 dword */
    pub number_of_players: u32,
    /* 1 dword */
    pub game_type: u32,
    /* 1 dword */
    pub language_id: u32,

    pub players: Vec<PlayerRecord>,

    pub game_record: GameRecord,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerRecord {
    /* 1 byte, always 0x16 */
    pub record_id: u8,
    /* 1 byte */
    pub player_id: u8,

    pub player_name: String,
    /* 1 byte */
    pub additional_data_size: u8,
    pub additional_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GameRecord {
    /* 1 byte, always 0x19 */
    pub record_id: u8,
    /* 1 word */
    pub num_data_bytes: u16,
    /* 1 byte */
    pub num_slot_records: u8,
    pub slot_records: Vec<SlotRecord>,
    /* 1 dword */
    pub random_seed: u32,
    /* 1 byte */
    pub select_mode: u8,
    /* 1 byte */
    pub start_spot_count: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SlotRecord {
    /* 1 byte */
    pub player_id: u8,
    /* 1 byte */
    pub download_percent: u8,
    /* 1 byte */
    pub slot_status: u8,
    /* 1 byte */
    pub player_flag: u8,
    /* 1 byte */
    pub team_number: u8,
    /* 1 byte */
    pub color: u8,
    /* 1 byte */
    pub race: u8,
    /* 1 byte */
    pub ai_strength: u8,
    /* 1 byte */
    pub handicap: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ReplayBlock
{
    /* The id takes up 1 byte */
    /* 0x10, value is parsed earler as "GameBlock" */ 
    /* 0x16, value is parsed earlier as "Players" */ 

    /* 0x17 */
    LeaveGame {
        /* 1 dword */
        reason: u32,
        /* 1 byte */
        player_id: u8,
        /* 1 dword */
        result: u32,
        /* 1 dword */
        session_leave_count: u32,
    },
    /* 0x1A */
    LoadStarted1 {
        /* 1 dword */
        unknown: u32,
    },
    /* 0x1B */
    LoadStarted2 {
        /* 1 dword */
        unknown: u32,
    },
    /* 0x1C */
    GameStarted {
        /* 1 dword */
        unknown: u32,
    },
    /* 0x1E */
    TickPreOverflow {
        /* 1 word */
        num_bytes: u16,
        /* 1 word */
        time_increment: u16,
        commands: Vec<Command>,
    },
    /* 0x1F */
    Tick {
        /* 1 word */
        num_bytes: u16,
        /* 1 word */
        time_increment: u16,
        commands: Vec<Command>,
        
    },
    /* 0x20 */
    PlayerChat {
        /* 1 byte */
        player_id: u8,
        /* 2 bytes */
        num_bytes: u16,
        /* 1 byte */
        flags: u8,
        /* 4 bytes */
        chat_mode: u32,
        message: String,
    },
    /* 0x22 */
    RandomSeed {
        /* 1 byte */
        num_bytes: u8,
        /* 4 bytes */
        unknown: u32,
    },
    /* 0x23 */
    Desync {
        /* 4 bytes */
        tick_count: u32,
        /* 4 bytes */
        checksum: u32,
        /* 1 bytes */
        remaining_players: u8,
    },
    /* 0x2F */
    ForceGameEndCountdown {
        /* 4 bytes */
        mode: u32,
        /* 4 bytes */
        time: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Command {
    /* 1 byte */
    pub player_id: u8,
    /* 1 word */
    pub num_bytes: u16,
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum GameSpeed
{ 
    Slow = 0, 
    Normal = 1, 
    Fast = 2,
}

impl GameSpeed
{
    fn from_u8(byte: u8) -> Result<GameSpeed>
    {
        match byte
        {
            0x0 => Ok(GameSpeed::Slow),
            0x1 => Ok(GameSpeed::Normal),
            0x2 => Ok(GameSpeed::Fast),
            _ => bail!(format!("{} is not a GameSpeed", byte)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum OrderType
{ 
    Queue = 0b1, 
    Train = 0b10, 
    Construct = 0b100, 
    Group = 0b1000, 
    NoFormation = 0b1_0000, 
    Summon = 0b10_0000,

    AutoCastOn = 0b1000_0000,
}

impl OrderType
{
    fn from_u16(byte: u16) -> Result<Vec<OrderType>>
    {
        let mut flags = vec![ OrderType::Queue
                            , OrderType::Train
                            , OrderType::Construct
                            , OrderType::Group
                            , OrderType::NoFormation
                            , OrderType::Summon
                            , OrderType::AutoCastOn
                            ];

        for x in (0..flags.len()).rev()
        {
            let flag = *flags.get(x)
                             .ok_or(format!("{} was not a valid index. len: {}",x,flags.len()))?
                             as u16;
            if byte & flag == 0
            {
                flags.remove(x);
            }
        }
        
        Ok(flags)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum SelectionOperation
{ 
    Add = 1,
    Remove = 2,
}

impl SelectionOperation
{
    fn from_u8(byte: u8) -> Result<SelectionOperation>
    {
        match byte
        { 
            1 => Ok(SelectionOperation::Add),
            2 => Ok(SelectionOperation::Remove),
            _ => bail!(format!("{} is not a SelectionOperation", byte)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum AllianceType
{
    Passive = 0b1,
    HelpRequest = 0b10,
    HelpResponse = 0b100,
    SharedXP = 0b1000,
    SharedSpells = 0b1_0000,
    SharedVision = 0b10_0000,
    SharedControl = 0b100_0000,
    FullSharedControl = 0b1000_0000,
    Rescuable = 0b1_0000_0000,
    SharedVisionForced = 0b10_0000_0000,
    AlliedVictory = 0b100_0000_0000,
}

impl AllianceType
{
    fn from_u32(byte: u32) -> Result<Vec<AllianceType>>
    {
        let mut flags = vec![ AllianceType::Passive
                            , AllianceType::HelpRequest
                            , AllianceType::HelpResponse
                            , AllianceType::SharedXP
                            , AllianceType::SharedSpells
                            , AllianceType::SharedVision
                            , AllianceType::SharedControl
                            , AllianceType::FullSharedControl
                            , AllianceType::Rescuable
                            , AllianceType::SharedVisionForced
                            , AllianceType::AlliedVictory
                            ];

        for x in (0..flags.len()).rev()
        {
            let flag = *flags.get(x)
                            .ok_or(format!("{} was not a valid index. len: {}",x,flags.len()))? 
                            as u32;
            if byte & flag == 0
            {
                flags.remove(x);
            }
        }
        
        Ok(flags)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum ArrowKeyEvent
{ 
    PressedLeftArrow = 0,
    ReleasedLeftArrow = 1,
    PressedRightArrow = 2,
    ReleasedRightArrow = 3,
    PressedDownArrow = 4,
    ReleasedDownArrow = 5,
    PressedUpArrow = 6,
    ReleasedUpArrow = 7,
}

impl ArrowKeyEvent
{
    fn from_u8(byte: u8) -> Result<ArrowKeyEvent>
    {
        match byte
        {
            0 => Ok(ArrowKeyEvent::PressedLeftArrow),
            1 => Ok(ArrowKeyEvent::ReleasedLeftArrow),
            2 => Ok(ArrowKeyEvent::PressedRightArrow),
            3 => Ok(ArrowKeyEvent::ReleasedRightArrow),
            4 => Ok(ArrowKeyEvent::PressedDownArrow),
            5 => Ok(ArrowKeyEvent::ReleasedDownArrow),
            6 => Ok(ArrowKeyEvent::PressedUpArrow),
            7 => Ok(ArrowKeyEvent::ReleasedUpArrow), 
            _ => bail!(format!("{} is not a ArrowKeyEvent", byte)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GameObject
{
    allocated_id: u32,
    counter_id: u32,
}

impl GameObject
{
    fn new(allocated_id: u32, counter_id: u32) -> GameObject
    {
        GameObject {
            allocated_id,
            counter_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnitInventory
{
    item: u32,
    charges: u32,
    unknown: u32,
}

impl UnitInventory
{
    fn new(item: u32, charges: u32, unknown: u32) -> UnitInventory
    {
        UnitInventory {
            item,
            charges,
            unknown,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnitAbility
{
    ability: u32,
    level: u32, 
}

impl UnitAbility
{
    fn new(ability: u32, level: u32) -> UnitAbility
    {
        UnitAbility {
            ability,
            level,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Action
{
    /* The id takes up 1 byte */

    /* 0x01 */
    PauseGame(),
    /* 0x02 */
    ResumeGame(),
    /* 0x03 */
    SetGameSpeed {
        /* 1 byte */
        speed: GameSpeed,
    },
    /* 0x04 */
    IncreaseGameSpeed(),
    /* 0x05 */
    DecreaseGameSpeed(),
    /* 0x06 */
    SaveGame {
        game_name: String,
    },
    /* 0x07 */
    SaveGameFinish {
        /* 4 bytes */
        unknown: u32,
    },
    /* 0x10 */
    SelfOrder {
        /* 2 bytes */
        flags: Vec<OrderType>, 
        order_id: u32, 
        unknown: GameObject, 
    },
    /* 0x11 */
    PointOrder {
        /* 2 bytes */
        flags: Vec<OrderType>, 
        order_id: u32, 
        unknown: GameObject, 
        x: f32, 
        y: f32, 
    },
    /* 0x12 */
    ObjectOrder {
        /* 2 bytes */
        flags: Vec<OrderType>, 
        order_id: u32, 
        unknown: GameObject, 
        x: f32, 
        y: f32,
        target: GameObject, 
    },
    /* 0x13 */
    DropOrGiveItem {
        /* 2 bytes */
        flags: Vec<OrderType>, 
        order_id: u32, 
        unknown: GameObject, 
        x: f32, 
        y: f32,
        receiver: GameObject,
        item: GameObject,
    },
    /* 0x14 */
    FogObjectOrder {
        /* 2 bytes */
        flags: Vec<OrderType>, 
        order_id: u32, 
        unknown: GameObject, 
        x: f32, 
        y: f32,
        target_type: u32,
        target_flags: u64,
        target_owner: u8,
        target_x: f32,
        target_y: f32, 
    },
    /* 0x16 */
    ChangeSelection { 
        select_mode: SelectionOperation, 
        targets: Vec<GameObject>,
    },
    /* 0x17 */
    AssignGroup { 
        group_number: u8, 
        targets: Vec<GameObject>
    },
    /* 0x18 */
    SelectGroup {
        /* 1 byte */
        group_number: u8,
        /* 1 byte */
        unknown: u8,
    },
    /* 0x19 */
    SelectSubGroup {
        /* 4 bytes */
        item_id: u32,
        target: GameObject,
    },
    /* 0x1A */
    PreSubSelection(),
    /* 0x1B */
    TriggerSelectionEvent {
        operation: SelectionOperation,
        target: GameObject,
    },
    /* 0x1C */
    SelectGroundItem {
        /* 1 byte */
        flags: u8,
        target: GameObject,
    },
    /* 0x1D */
    CancelHeroRevival {
        target: GameObject,
    },
    /* 0x1E */
    CancelUnitInQueue { 
        slot_index: u8, 
        unit_id: u32,
    },
    /* 0x21 */
    Unknown21 {
        /* 4 bytes */
        unknown_a: u32,
        /* 4 bytes */
        unknown_b: u32,
    },
    /* 0x20 */
    CheatTheDudeAbides(),
    /* 0x22 */
    CheatSomebodySetUpUsTheBomb(),
    /* 0x23 */
    CheatWarpTen(),
    /* 0x24 */
    CheatIocainePowder(),
    /* 0x25 */
    CheatPointBreak(),
    /* 0x26 */
    CheatWhosYourDaddy(),
    /* 0x27 */ 
    CheatKeyserSoze { 
        /* 1 byte */
        unknown: u8,
        /* 4 bytes */
        gold: i32,
    },
    /* 0x28 */ 
    CheatLeafItToMe { 
        /* 1 byte */
        unknown: u8,
        /* 4 bytes */
        lumber: i32,
    },
    /* 0x29 */
    CheatThereIsNoSpoon(),
    /* 0x2A */
    CheatStrengthAndHonor(),
    /* 0x2B */
    CheatItVexesMe(),
    /* 0x2C */
    CheatWhoIsJohnGalt(),
    /* 0x2D */
    CheatGreedIsGood { 
        /* 1 byte */
        unknown: u8,
        /* 4 bytes */
        resources: i32,
    },
    /* 0x2E */
    CheatDaylightSavings {
        /* 4 bytes */
        time: f32,
    },
    /* 0x2F */
    CheatISeeDeadPeople(),
    /* 0x30 */
    CheatSynergy(),
    /*0x31 */
    CheatSharpAndShiny(),
    /* 0x32 */
    CheatAllYourBaseAreBelongToUs(),
    /* 0x50 */
    ChangeAlly { 
        player_id: u8, 
        flags: Vec<AllianceType>,
    },
    /* 0x51 */
    TransferResources {
        /* 1 byte */
        player_id: u8,
        /* 4 bytes */
        gold_transfered: i32,
        /* 4 bytes */
        lumber_transfered: i32,
    },
    /* 0x60 */
    MapTriggerChat {
        event: GameObject,
        message: String,
    },
    /* 0x61 */
    Esc(),
    /* 0x62 */
    TriggerSleepOrSyncFinished {
        thread: GameObject,
        wait_count: u32,
    },
    /* 0x63 */
    TriggerSyncReady {
        thread: GameObject,
    },
    /* 0x64 */
    TriggerMouseClickedTrackable{
        trackable: GameObject,
    },
    /* 0x65 */
    TriggerMouseTouchedTrackable{
        trackable: GameObject,
    },
    /* 0x66 */
    EnterHeroSkillSubMenu(),
    /* 0x67 */
    EnterBuildingSubMenu(),
    /* 0x68 */
    MiniMapSignal {
        /* 4 bytes */
        location_x: f32,
        /* 4 bytes */
        location_y: f32,
        /* 4 bytes */
        duration: f32,
    },
    /* 0x69 */
    DialogButtonClicked {
        dialog: GameObject,
        button: GameObject, 
    },
    /* 0x6A */
    DialogAnyButtonClicked {
        button: GameObject,
        dialog: GameObject, 
    },
    /* 0x6B */
    SyncStoredInteger {
        file: String,
        group: String,
        key: String,
        /* 4 bytes */
        value: i32,
    }, 
    /* 0x6C */
    SyncStoredFloat {
        file: String,
        group: String,
        key: String,
        /* 4 bytes */
        value: f32,
    },
    /* 0x6D */
    SyncStoredBoolean {
        file: String,
        group: String,
        key: String,
        /* 4 bytes */
        value: u32,
    },
    /* 0x6E */
    SyncStoredUnit {
        file: String,
        group: String,
        key: String, 
        unit_type: u32,
        /* u32 for inventory size */
        inventory: Vec<UnitInventory>,
        experience: u32,
        level_ups: u32,
        skill_points: u32,
        proper_name_index: u16,
        unknown1: u16,
        base_strength: u32,
        bonus_strength_per_level: f32,
        base_agility: u32,
        bonus_move_speed: f32,
        bonus_attack_speed: f32,
        bonus_agility_per_level: f32,
        base_intelligence: u32,
        bonus_intelligence_per_level: f32,
        /* u32 for ability size */
        abilities: Vec<UnitAbility>, 
        bonus_health: f32,
        bonus_mana: f32,
        sight_radius_day: f32,
        unknown2: u32,
        unknown3: u32,
        unknown4: u32,
        unknown5: u32,
        hotkey_flags: u16,
    },
    /* 0x6F */
    SyncStoredString {
        file: String,
        group: String,
        key: String,
        value: String,
    },
    /* 0x70 */
    SyncEmptyInteger {
        file: String,
        group: String,
        key: String, 
    }, 
    /* 0x71 */
    SyncEmptyString {
        file: String,
        group: String,
        key: String, 
    },
    /* 0x72 */
    SyncEmptyBoolean {
        file: String,
        group: String,
        key: String, 
    },
    /* 0x73 */
    SyncEmptyUnit {
        file: String,
        group: String,
        key: String, 
    },
    /* 0x74 */
    SyncEmptyFloat {
        file: String,
        group: String,
        key: String, 
    },
    /* 0x75 */
    TriggerArrow { 
        key: ArrowKeyEvent,
    }
}