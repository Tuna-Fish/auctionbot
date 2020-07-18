DROP TABLE IF EXISTS perks;
CREATE TABLE perks (
	name VARCHAR(30) PRIMARY KEY,
	day INTEGER,
	nr INTEGER,
	descr TEXT
);

INSERT INTO perks (day, nr, name, descr) VALUES
	(4,1, 'GEN_NONCAP_SACRED', 'Your nation is generated with a rec-everywhere sacred.'),
	(4,2, 'GEN_HINF_SACRED', 'Your nation is generated with a heavy infantry sacred (min 18 prot).'),
	(4,3, 'GEN_HCAV_SACRED', 'Your nation is generated with a heavy cavalry sacred (min 15 prot and a lance).'),
	(4,4, 'GEN_NONCAP_H3','Your nation is generated with a rec-everywhere H3 priest.'),
	(4,5, 'GEN_HCHARIOT_SACRED','Your nation is generated with a heavy chariot sacred (min 15 prot).'),
	(4,6, 'CHEAP_CAPONLY_RES_REC','All non-gold costs for all your cap-only units and commanders  get halved. Applies after all other rules.'),
	
	(4,7, 'ASTRAL_SPELLS', 'You get Celestial Music, Yazads, Gandharvas, Celestial Yazads, Amesha Spentas and Seraphs(heavenly choir).'),
	(4,8, '1REC_POINT_WEAK_MAGE','Your weakest mages cost one rec point, and get -2rp.'),
	(4,9, 'NONCAP_COMMUNICANT','Rec-everywhere national non-sacred communion slave troop with 1 rec limit.'),
	(4,10,'PRIESTTHUG','All your priests with no magic paths get +10 hp, +3 str, +4 att/def, and basic combat gear.'),
	(4,11,'CONJURERCIRCLE',E'You get conjurer\'s circle (20% conj discount) as a cap site.'),
	(4,12,'FORGEBONUS', 'Your big mages get +2 forge bonus.'),
		
	(5,1, 'BERSERKERSACRED','Your sacreds get +5 berserker.'),
	(5,2, 'CONVERT_SACRED_NONCAP','Your strongest sacred is converted to rec-everywhere.'),
	(5,3, 'STEELOVENS','You get steel ovens as a cap site.'),
	(5,4, 'PRIESTCOMSLAVE','Your weakest national priest is a communion slave.'),
	(5,5, 'MAGETHUG', 'Your big mages are made not old, get +10hp, +4str, +5 att/def, and get normal sacred-tier combat gear.'),
	(5,6, 'UNICORNPOWER', 'Your best mounted unit will ride on unicorns. (Horn attack, fast, recuperation.) You also get a themed flag.'),
	
	(5,7, '2RANK_WEAK_MAGE', 'Your weakest mages have their paths set to exactly 2 ranks of your primary path, and cost reset to what a 1/1 would cost'),
	(5,8, 'SACRED_HEAVY_MAGIC_ARMOR', 'Your most powerful sacreds get heavy magic armor. No effects, just better-than-normal stats.'),
	(5,9, 'TOOMANY_BLESSPOINTS','You get +5 bless points for your primary path.'),
	(5,10,'NATURE_SPELLS','You get Fort of the Ancients, and can summon Lilots, Mazzikim, Balam and Mountain Vila. You can also send monster Boars.'),
	(5,11,'SPACEJAM','Your national sacreds get a rock-throwing attack with AOE. You get those as wall defenders and national pd. You also get a themed flag.'),
	(5,12,'NONCAP_BIG_MAGES', 'Your big mages will not be cap-only.'),

	(6,1, 'CONVERT_ALL_INTO_SACREDS_LOL','All your national units and commanders are converted to sacreds. Temples cost +200g'),
	(6,2, 'REANIMATOR_PRIESTS','All your primary race priests can reanimate undead.'),
	(6,3, '3RANK_MEDIUM_MAGE',E'One of your medium mages gets it\'s paths reset to exactly 3 ranks in your primary path, and cost reset to what a 2/2 would cost'),
	(6,4, 'SACRED_BEST_MAGIC_WEAPON','Your best sacreds get the best magical weapons of their type I see out of natgen.'),
	(6,5, 'MAGEPRIESTS','All your national mages will be sacred and have Holy paths. Big mages get H3, other less.'),
	(6,6, 'WATER_SUMMONS','You get the spells for Kaijin, Olm Conclave, Living Mercury, and Telkhines.'),
	
	(6,7, 'BEEFCAKE_SACREDS','All your sacred troops get +10hp and +4str'),
	(6,8, 'SACRIFICIAL_GROVE','You get sacrificial grove (20% blood discount) as an additional cap site'),
	(6,9, 'CAP_ONLY_DEATHREC','All your cap-only units get a death recruit bonus that cuts their cost in half in Death 3 dominion.'),
	(6,10,'SLAVE_TROOPS','I generate a few slave troops of any race that was not picked. Your normal commanders get taskmaster.'),
	(6,11,'SNIPER_MAGES','Your national mages get +5 prec.'),
	(6,12,'TURLE_POWER','You get badass heavily armored wingless arbalesters with op stats as national pd and wall defenders. You cannot recruit them. You also get a themed flag.'),


	(7,1, 'CONVERT_WEAKEST_INTO_SACRED','I will convert your weakest unit into a sacred.'),
	(7,2, 'MAGIC_WEAPON','Whatever weapon your sacreds have gets turned into magic ones'),
	(7,3, 'COMMUNION_COPY','You get copies of the communion spells in your primary path.'),
	(7,4, 'BLACKSTEEL_INFANTRY','You get a national copy of Ulmish plate infantry with flails.'),
	(7,5, 'GACHA_MAGES','All your national mages get paths based on non-linked randoms.'),
	(7,6, 'DEATH_SUMMONS','You get the summon spells for Morrigans, Ancestor Spirits, Bean Sidhes, and GRAND LEMURS. The Lemur spell is converted to be available at Conj 9.'),
	
	(7,7, 'BLOOD_SUMMONS',E'You get the summon spells for Se\'ir, Sandhyabala, Fallen Angels, and Grigori. Note that most of these spells get made more expensive by JBBM.'),
	(7,8, 'ADVANCED_FORTS',E'You get advanced forts.'),
	(7,9, 'VERY_PRIMITIVE_FORTS','You only get palisades, but you get a 70% discount on them. You cannot take both this and advanced forts.'),
	(7,10,'SPELLSINGER','All your national mages are spellsingers.'),
	(7,11,'SKILLED_SACREDS','All your sacred troops get +3 att/def/prec'),
	(7,12,'GEMGEN_TEMPLES','All your temples produce gems of your primary path, like LA Ragha. Your temples cost +200g'),

	(8,1, 'PICK_ANY_PRETENDER','Your choice of any available pretender chassis in JBBM (no bronze colossus).'),
	(8,2, 'FIRE_SPELLS','You get summon spells for Firebirds, Jinn warriors, Jinns and Marids. You also get smokeless fire, and liquid flames of Rhuax.'),
	(8,3, 'HEALER_PRIESTS','Your strongest rec-everywhere priest has healer 2x their priest value'),
	(8,4, 'FORTUNE', 'All your national mages get fortuneteller 3.'),
	(8,5, 'ELEGISTs','Your weakest priests get 2 points of elegist and your god will not lose paths on death'),
	(8,6, 'EARTH_SPELLS','You get ulmish antimagic darts, and also sentinel statues, granite guardians, marble oracles and iron angels.'),
	
	(8,7, 'SLAVE_MAGES','You get a weak and a medium mage as slaves from any race that was not picked'),
	(8,8, 'DUAL_WIELDING_SACREDS', 'Your sacreds get ambidextrous and two unremarkable short one-handed weapons.'),
	(8,9, 'AIR_SUMMONS', 'You get Simargls, Condors, Dai tengus and Chaac.'),
	(8,10,'UNHOLY BUFFS', 'You get the undead holy buff spells.'),
	(8,11,'CAP_ONLY_COM_SLAVE','You get cap-only, sacred, rec limit 3 communion slave troops.'),
	(8,12,'DRAINIMMUNE','All your national mages are immune to effects of drain dominion')
