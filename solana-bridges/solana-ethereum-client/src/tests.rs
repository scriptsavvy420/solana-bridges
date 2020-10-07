#![allow(dead_code)]

use quickcheck_macros::quickcheck;

use crate::{
    instruction::*,
    processor::*,
};

use solana_sdk::{
    account_info::{AccountInfo},
    pubkey::Pubkey,
    program_error::ProgramError,
    program_pack::{Pack},
};

// Required to support info! in tests
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

mod test {
    use super::*;
    use crate::eth::*;
    use crate::parameters::MIN_BUF_SIZE;
    use solana_sdk::clock::Epoch;
    use std::str::FromStr;
    use rlp::{Decodable, Encodable, Rlp};
    use ethereum_types::{U256, H64, H160, H256, Bloom};
    use hex_literal::hex;

    #[quickcheck]
    fn test_instructions(mut buf_len: usize) -> Result<(), TestError> {
        if buf_len <= MIN_BUF_SIZE {
            buf_len += MIN_BUF_SIZE;
        }
        let header_400000 = decode_rlp(&hex_to_bytes(HEADER_400000)?)?;
        let header_400001 = decode_rlp(&hex_to_bytes(HEADER_400001)?)?;

        let block_400000 = Block { header: header_400000, transactions: Vec::new() };
        let block_400001 = Block { header: header_400001, transactions: Vec::new() };

        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut lamports = 0;
        let mut raw_data = vec![0; buf_len];

        let owner = Pubkey::default();
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut raw_data,
            &owner,
            false,
            Epoch::default(),
        );

        assert_eq!(block_400000.transactions.len(), 0);
        let instruction_noop: Vec<u8> = Instruction::Noop.pack();
        let instruction_init: Vec<u8> = Instruction::Initialize(block_400000.header).pack();
        let instruction_new: Vec<u8> = Instruction::NewBlock(block_400001.header).pack();

        let accounts = vec![account];
        process_instruction(&program_id, &accounts, &instruction_noop).map_err(TestError::ProgError)?;
        process_instruction(&program_id, &accounts, &instruction_init).map_err(TestError::ProgError)?;
        process_instruction(&program_id, &accounts, &instruction_new).map_err(TestError::ProgError)?;

        let data = interp(&*raw_data);
        assert_eq!(normalize_count(data, 2), data.count);
        assert_eq!(400001, data.height);
        return Ok(());
    }

    fn test_header_pow(header: &str) -> Result<(), TestError> {
        assert_eq!(true, verify_pow(&decode_rlp(&hex_to_bytes(header)?)?));
        return Ok(());
    }

    // Slow tests ~ 1min each
    //#[test]
    fn test_pow() -> Result<(), TestError> {
        test_header_pow(HEADER_400000)?;
        test_header_pow(HEADER_400001)?;
        test_header_pow(HEADER_8996776)?;
        return Ok (());
    }


    fn test_extradata_pack(extra: ExtraData) -> Result<(), TestError> {
        let mut extra_slice = [0; ExtraData::LEN];
        extra.pack_into_slice(&mut extra_slice);
        assert_eq!(extra.bytes.len() as u8, extra_slice[0]);
        assert_eq!(extra, ExtraData::unpack_from_slice(&extra_slice).map_err(TestError::ProgError)?);
        return Ok(());
    }

    #[test]
    fn test_roundtrip_pack() -> Result<(), TestError> {
        test_extradata_pack(ExtraData { bytes: vec![] })?;
        test_extradata_pack(ExtraData { bytes: vec![4] })?;
        test_extradata_pack(ExtraData { bytes: vec![5,5] })?;
        test_extradata_pack(ExtraData { bytes: vec![6,6,6] })?;

        let expected = decoded_header_0()?;
        let mut buffer = [0; BlockHeader::LEN];
        expected.pack_into_slice(&mut buffer);
        let unpacked = BlockHeader::unpack_from_slice(&buffer).map_err(TestError::ProgError)?;
        assert_eq!(expected, unpacked);

        return Ok(());
    }

    #[test]
    fn test_roundtrip_rlp() -> Result<(), TestError> {
        let expected = decoded_header_0()?;
        assert_eq!(expected, decode_rlp(&encode_header(&expected))?);
        return Ok(());
    }

    #[test]
    fn test_decoding() -> Result<(), TestError> {
        let expected = decoded_header_0()?;
        let header: BlockHeader = decode_rlp(&hex_to_bytes(TEST_HEADER_0)?)?;
        assert_eq!(header, expected);

        let header_400k: BlockHeader = decode_rlp(&hex_to_bytes(HEADER_400000)?)?;
        assert_eq!(header_400k.number, 400000);
        assert_eq!(header_400k.difficulty, U256::from(6022643743806 as u64));
        assert_eq!(hash_header(&header_400k, false), H256::from_str("5d15649e25d8f3e2c0374946078539d200710afc977cdfc6a977bd23f20fa8e8").map_err(|_| TestError::HexError)?);

        let test_block_0_tx: Block = decode_rlp(TEST_BLOCK_0_TX)?;
        assert_eq!(test_block_0_tx.header.number, 4);

        let test_block_1_tx: Block = decode_rlp(TEST_BLOCK_1_TX)?;
        assert_eq!(test_block_1_tx.header.number, 2);
        assert_eq!(test_block_1_tx.transactions.len(), 1);

        return Ok(());
    }

    #[derive(Debug)]
    enum TestError {
        HexError,
        RlpError,
        ProgError(ProgramError),
    }

    fn hex_to_bytes(h: &str) -> Result<Vec<u8>, TestError> {
        return hex::decode(h).map_err(|_| TestError::HexError);
    }
    fn decode_rlp <T:Decodable> (bytes: &[u8]) -> Result<T, TestError> {
        let rlp = Rlp::new(bytes);
        return T::decode(&rlp).map_err(|_| TestError::RlpError);
    }
    fn encode_header(header: &BlockHeader) -> Vec<u8> {
        return header.rlp_bytes();
    }


    const HEADER_400000: &str = "f90213a01e77d8f1267348b516ebc4f4da1e2aa59f85f0cbd853949500ffac8bfc38ba14a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347942a65aca4d5fc5b5c859090a6c34d164135398226a00b5e4386680f43c224c5c037efc0b645c8e1c3f6b30da0eec07272b4e6f8cd89a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421b901000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000086057a418a7c3e83061a80832fefd880845622efdc96d583010202844765746885676f312e35856c696e7578a03fbea7af642a4e20cd93a945a1f5e23bd72fc5261153e09102cf718980aeff38886af23caae95692ef";

    const HEADER_400001: &str = "f90215a05d15649e25d8f3e2c0374946078539d200710afc977cdfc6a977bd23f20fa8e8a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d493479452bc44d5378309ee2abf1539bf71de1b7d7be3b5a09aeed0f1a990a5578fbe75d4404f3011ff8b4c108cb8c5a634e499d153d28488a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421b901000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000086057af0d2ad9183061a81832fefd880845622efe498d783010202844765746887676f312e342e32856c696e7578a0729654a37843e931a3680a27360115ae0d2f902110e1def46975f651f2e7becb8849ef7c60937788e9";

    const HEADER_8996776: &str = "f90215a0f28520c0b577aa94d27bfd84ac15b9a1bd0c97815ca086935fbb6f6fe69681c9a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d4934794ea674fdde714fd979de3edf0f56aa9716b898ec8a0c1277d7c2d1dddedf1b9a49f304bef09f65b531e94cbad2629a7fa88a22690eea08c65778bbc912fd96e116b869a5ffc9b671e473a9872d8edb68cd1b613200d16a0a82e3b139ea78960b0bd858667e067ab9b161a9287aebf5927afd3346fe91ab2b901000c0b52d1276a3048372233e31022d9941b94a599117b481f800079c0921954086c480b076f95da0ef0a011839293035452bb26d30f885a014028c88478283a10c1a5c0b2b131e1896d36105a248068e026366d94948a000c0b7a335c22c03dd85656d90a0e14500cf531431223812a330c007a352608d53029658174090052127d002f2dda01600b962c9421853103940c5199f4436132446f73018eb07468c06a002881a4042080348083d090be5101296720195195083110a942849ac4282718f2520223cab1a2080eb21047a415669e40165187e3109449c4368ada546022a21064781945a9ed804068001815a812984310088012000174b43f5e28f9e0bd87092aa28cbc4930838947a88398833e83983217845ddb678f94505059452d65746865726d696e652d6575312d38a0a1b6535bc565ed913565f8c471ec88ed73f8d59c61009c148913c791a4e3e168887ac6c6600610c8fb";

    const TEST_HEADER_0: &str = "f9021aa0f779e50b45bc27e4ed236840e5dbcf7afab50beaf553be56bf76da977e10cc73a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d493479452bc44d5378309ee2abf1539bf71de1b7d7be3b5a014c996b6934d7991643669e145b8355c63aa02cbde63d390fcf4e6181d5eea45a079b7e79dc739c31662fe6f25f65bf5a5d14299c7a7aa42c3f75b9fb05474f54ca0e28dc05418692cb7baab7e7f85c1dedb8791c275b797ea3b1ffcaec5ef2aa271b9010000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000010000000000000000000000000000000000000000000000000000000408000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000010000000000000000000000000000000000000000000000000000000400000000000100000000000000000000000000080000000000000000000000000000000000000000000100002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000903234373439353837313930323034343383890fe68395ba8e82d0d9845dd84a079150505945206e616e6f706f6f6c2e6f7267a0a35425f443452cf94ba4b698b00fd7b3ff4fc671dea3d5cc2dcbedbc3766f45e88af7fec6031063a17";

    const TEST_BLOCK_1_TX: &[u8] = &hex!("f904eaf90213a0c89928efed5db6530c482c236da3aaeaba6435a2450a975e9b9f1f5ff6941723a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347940000000000000000000000000000000000000001a0f0bf02aac82e0961d87a128569740012d6e2ec99a395157ba97709a9de950fe2a04e4964659ef22d9ecee734c5f7b8bcd00680b6329206da84ae388c383f905cb0a0777f1c1c378807634128348e4f0eeca6a0e7f516ea411690ca04266323f671a4b90100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008302004002833007cf830186a0845f7b5d9399d883010914846765746888676f312e31352e31856c696e7578a0c4bb1584988635f3c191eb599e2c05f450488df962904171a5547ead9131e3f9881450280dc437cf3cf902d0f902cd8001830186a08001b9027c3630383036303430353233343830313536313030313035373630303038306664356235303631303131653830363130303230363030303339363030306633666536303830363034303532333438303135363030663537363030303830666435623530363030343336313036303238353736303030333536306530316338303633633630356637366331343630326435373562363030303830666435623630333336306162353635623630343035313830383036303230303138323831303338323532383338313831353138313532363032303031393135303830353139303630323030313930383038333833363030303562383338313130313536303731353738303832303135313831383430313532363032303831303139303530363035383536356235303530353035303930353039303831303139303630316631363830313536303964353738303832303338303531363030313833363032303033363130313030306130333139313638313532363032303031393135303562353039323530353035303630343035313830393130333930663335623630363036303430353138303630343030313630343035323830363030643831353236303230303137663438363536633663366632633230353736663732366336343231303030303030303030303030303030303030303030303030303030303030303030303030303038313532353039303530393035366665613236343639373036363733353832323132323063346466366139393637666230336633323038653966383534623236643635626338343665323134393963646363333135303639313431653530623036623165363437333666366336333433303030363038303033338325ad31a06be9f7bacbbc298818438802d6c202df6084649643afce090e017f1cb37c3618a031fc123f349bdb40ccf39a159a31810d0cc6cff00a920a75c4d97cad8c36c938c0");

    const TEST_BLOCK_0_TX: &[u8] = &hex!("f90215f90210a0d08f55a1789e660d82802ae3970130beb9736fc1b36c2e15f4589467dbdea06ca01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347940000000000000000000000000000000000000001a0c4f67a7baaa163869ad9461c00cca706e317ca4e4ff4e22e4843ae2af0960003a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000830200c00483301fd280845f7b762399d883010914846765746888676f312e31352e31856c696e7578a059249dd6f99033815acf89b66da4b44e5e93d588e85e4549009152f4a513601b884983ec06833e1e43c0c0");

    fn decoded_header_0() -> Result<BlockHeader, TestError> {
        let expected = BlockHeader {
            parent_hash: H256::from([
                0xf7, 0x79, 0xe5, 0x0b, 0x45, 0xbc, 0x27, 0xe4,
                0xed, 0x23, 0x68, 0x40, 0xe5, 0xdb, 0xcf, 0x7a,
                0xfa, 0xb5, 0x0b, 0xea, 0xf5, 0x53, 0xbe, 0x56,
                0xbf, 0x76, 0xda, 0x97, 0x7e, 0x10, 0xcc, 0x73,
            ]),
            uncles_hash: H256::from([
                0x1d, 0xcc, 0x4d, 0xe8, 0xde, 0xc7, 0x5d, 0x7a,
                0xab, 0x85, 0xb5, 0x67, 0xb6, 0xcc, 0xd4, 0x1a,
                0xd3, 0x12, 0x45, 0x1b, 0x94, 0x8a, 0x74, 0x13,
                0xf0, 0xa1, 0x42, 0xfd, 0x40, 0xd4, 0x93, 0x47,
            ]),
            author: H160::from([
                0x52, 0xbc, 0x44, 0xd5,
                0x37, 0x83, 0x09, 0xee,
                0x2a, 0xbf, 0x15, 0x39,
                0xbf, 0x71, 0xde, 0x1b,
                0x7d, 0x7b, 0xe3, 0xb5,
            ]),
            state_root: H256::from([
                0x14, 0xc9, 0x96, 0xb6, 0x93, 0x4d, 0x79, 0x91,
                0x64, 0x36, 0x69, 0xe1, 0x45, 0xb8, 0x35, 0x5c,
                0x63, 0xaa, 0x02, 0xcb, 0xde, 0x63, 0xd3, 0x90,
                0xfc, 0xf4, 0xe6, 0x18, 0x1d, 0x5e, 0xea, 0x45,
            ]),
            transactions_root: H256::from([
                0x79, 0xb7, 0xe7, 0x9d, 0xc7, 0x39, 0xc3, 0x16,
                0x62, 0xfe, 0x6f, 0x25, 0xf6, 0x5b, 0xf5, 0xa5,
                0xd1, 0x42, 0x99, 0xc7, 0xa7, 0xaa, 0x42, 0xc3,
                0xf7, 0x5b, 0x9f, 0xb0, 0x54, 0x74, 0xf5, 0x4c,
            ]),
            receipts_root: H256::from([
                0xe2, 0x8d, 0xc0, 0x54, 0x18, 0x69, 0x2c, 0xb7,
                0xba, 0xab, 0x7e, 0x7f, 0x85, 0xc1, 0xde, 0xdb,
                0x87, 0x91, 0xc2, 0x75, 0xb7, 0x97, 0xea, 0x3b,
                0x1f, 0xfc, 0xae, 0xc5, 0xef, 0x2a, 0xa2, 0x71,
            ]),
            log_bloom: Bloom::from([
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x10, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]),
            difficulty: U256::from_str("32343734393538373139303230343433").map_err(|_| TestError::HexError)?,
            number: 8982502,
            gas_limit: U256::from(9812622),
            gas_used: U256::from(53465),
            timestamp: 1574455815,
            extra_data: ExtraData { bytes: Vec::from([80, 80, 89, 69, 32, 110, 97, 110, 111, 112, 111, 111, 108, 46, 111, 114, 103]) },
            mix_hash: H256::from([
                0xa3, 0x54, 0x25, 0xf4, 0x43, 0x45, 0x2c, 0xf9,
                0x4b, 0xa4, 0xb6, 0x98, 0xb0, 0x0f, 0xd7, 0xb3,
                0xff, 0x4f, 0xc6, 0x71, 0xde, 0xa3, 0xd5, 0xcc,
                0x2d, 0xcb, 0xed, 0xbc, 0x37, 0x66, 0xf4, 0x5e,
            ]),
            nonce: H64::from([
                0xaf, 0x7f, 0xec, 0x60, 0x31, 0x06, 0x3a, 0x17,
            ]),
        };
        return Ok(expected);
    }
}
